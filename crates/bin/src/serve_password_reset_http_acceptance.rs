use crate::{
    password_reset_composition_support::router_with_identity_reset,
    password_reset_identity_support::{ok, Acceptance},
    password_reset_runtime_support::{self as rates, Fixture},
    serve_identity_composition,
    serve_password_reset_composition::open_with_delivery_factory,
    serve_password_reset_http_acceptance_support::*,
    serve_start::PasswordResetRuntime,
    serve_stop::Stop,
};
use application::{
    identity::{password_reset::PasswordResetDelivery, SecretProtector, SessionStore},
    ApplicationError,
};
use axum::http::StatusCode;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::{
    clock::Clock,
    crypto::{TotpProvider, RECOVERY_CODE_COUNT},
    identity::Role,
};
use infrastructure::{
    password_reset_email::PasswordResetEmailConfiguration, SystemClock, TotpRsProvider,
};
use std::{
    future::Future,
    num::NonZeroUsize,
    sync::{atomic::Ordering::SeqCst, Arc},
    task::{Context, Waker},
    time::Duration,
};
use web::{HttpLimits, HttpWorkBudget};
use zeroize::Zeroizing;

#[test]
fn composed_http_reset_uses_real_backends_and_requires_existing_mfa_after_completion() {
    let mut keys = Fixture::new();
    let mut flow = Acceptance::new();
    let owner_session = flow.owner_session();
    let (email, email_rate_key) = keys.email();
    flow.track_email(&email);
    let old_password = Zeroizing::new("initial password for HTTP acceptance".to_string());
    let new_password = Zeroizing::new("replacement password for HTTP acceptance".to_string());
    let enrolled = ok(
        flow.identity
            .create_user(&owner_session, &email, &old_password, Role::Litigator),
        "enroll HTTP target",
    );
    let id = enrolled.principal.id;
    flow.assign_case(id);

    let stop = Arc::new(Stop::default());
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    let (capture, release) = Capture::new();
    let capture = Arc::new(capture);
    let factory_capture = capture.clone();
    let components = open_with_delivery_factory(
        &flow.db.runtime_url,
        &keys.url,
        settings(),
        budget.clone(),
        stop.clone(),
        move |_key: Zeroizing<String>,
              _configuration: PasswordResetEmailConfiguration,
              _connect: Duration,
              _total: Duration|
              -> Result<Arc<dyn PasswordResetDelivery>, ApplicationError> {
            Ok(factory_capture)
        },
    )
    .unwrap_or_else(|_| panic!("open composed recovery"))
    .expect("recovery enabled");
    let identity = serve_identity_composition::open(
        &flow.db.runtime_url,
        &keys.url,
        Zeroizing::new(vec![42; 32]),
        flow.policy,
    )
    .unwrap_or_else(|_| panic!("open real HTTP identity"));
    let router = router_with_identity_reset(
        identity,
        components.http,
        HttpLimits {
            max_requests: NonZeroUsize::new(4).unwrap(),
            max_blocking_operations: NonZeroUsize::new(1).unwrap(),
        },
        budget.clone(),
    );
    let PasswordResetRuntime { owner, consumer } = components.runtime;
    // All synchronous owners are constructed and retained outside Tokio.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let consumer = runtime.spawn(consumer.run());

    let initial_challenge = login(&runtime, &router, &mut keys, &email, &old_password);
    let old_session = mfa(
        &runtime,
        &router,
        &mut keys,
        "recovery",
        &initial_challenge,
        &enrolled.recovery_codes[0],
    );
    assert_eq!(
        runtime.block_on(me(&router, &old_session)).status,
        StatusCode::OK
    );
    let old_totp = login(&runtime, &router, &mut keys, &email, &old_password);
    let old_recovery = login(&runtime, &router, &mut keys, &email, &old_password);
    let before = flow.snapshot(id);
    assert_eq!(before.remaining, RECOVERY_CODE_COUNT - 1);
    assert!(before.consumed(0));

    let occupied = runtime.block_on(budget.acquire_owned()).unwrap();
    let requested = runtime.block_on(post(
        &router,
        REQUEST,
        &[("email", &format!("  {}  ", email.to_ascii_uppercase()))],
    ));
    assert_eq!(requested.status, StatusCode::ACCEPTED);
    requested.reset_public(&[&email, &old_password, &new_password]);
    assert_eq!(capture.calls.load(SeqCst), 0);
    let count: i64 = flow
        .db
        .admin
        .query_one("SELECT count(*) FROM password_reset_capabilities", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0, "request admission bypassed occupied work budget");
    drop(occupied);
    let envelope = runtime.block_on(capture.take());
    assert_eq!(envelope.email, email);
    assert_eq!(envelope.token.len(), 32);
    assert!(envelope.expires_at > SystemClock::new().now());
    flow.assert_reset_digest(envelope.token.as_ref());
    let mut waiting = Box::pin(budget.acquire_owned());
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending(),
        "consumer did not retain the shared HTTP budget during delivery"
    );
    drop(waiting);
    let token = Zeroizing::new(URL_SAFE_NO_PAD.encode(envelope.token.as_ref()));
    assert_eq!(token.len(), 43);
    let completion_key = keys.track(rates::completion_key(reset_digest(envelope.token.as_ref())));
    release.send(()).unwrap();

    let completed = runtime.block_on(post(
        &router,
        COMPLETE,
        &[("token", &token), ("new_password", &new_password)],
    ));
    assert_eq!(completed.status, StatusCode::NO_CONTENT);
    assert!(
        completed.body.is_empty(),
        "completion must not create a session"
    );
    completed.reset_public(&[&email, &token, &new_password]);
    let after = flow.snapshot(id);
    assert!(
        before.same_preserved_identity(&after),
        "reset changed identity or MFA material"
    );
    assert!(
        before.hash.as_str() != after.hash.as_str(),
        "password hash unchanged"
    );
    assert!(after.hash.starts_with("$argon2id$v=19$"));
    assert!(after.hash.contains("m=262144,t=3,p=1"));
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.generation, before.generation + 1);
    let retained = ok(
        flow.sessions.find_session(&old_session, flow.policy),
        "read old Redis session",
    )
    .expect("old credential remains in Redis");
    assert_eq!(retained.identity.auth_generation, before.generation);
    runtime
        .block_on(me(&router, &old_session))
        .error(StatusCode::UNAUTHORIZED, "invalid_session");

    let secret = ok(
        flow.secrets.expose(id, &after.protected_totp),
        "expose unchanged TOTP",
    );
    let stale_code = Zeroizing::new(ok(
        TotpRsProvider::new().current_code(
            &secret,
            SystemClock::new().now().unix_timestamp().max(0) as u64,
        ),
        "create real TOTP",
    ));
    flow.track_totp(id, &stale_code);
    runtime
        .block_on(post(
            &router,
            "/api/v1/auth/mfa/totp",
            &[("challenge_token", &old_totp), ("code", &stale_code)],
        ))
        .error(StatusCode::UNAUTHORIZED, "mfa_rejected");
    runtime
        .block_on(post(
            &router,
            "/api/v1/auth/mfa/recovery",
            &[
                ("challenge_token", &old_recovery),
                ("code", &enrolled.recovery_codes[1]),
            ],
        ))
        .error(StatusCode::UNAUTHORIZED, "mfa_rejected");
    let denied = flow.snapshot(id);
    assert!(
        after.same_preserved_identity(&denied),
        "stale challenge changed MFA"
    );
    assert_eq!(denied.revision, after.revision);
    runtime
        .block_on(post(
            &router,
            "/api/v1/auth/login",
            &[("email", &email), ("password", &old_password)],
        ))
        .error(StatusCode::UNAUTHORIZED, "invalid_credentials");

    let challenge = login(&runtime, &router, &mut keys, &email, &new_password);
    let current_code = Zeroizing::new(ok(
        TotpRsProvider::new().current_code(
            &secret,
            SystemClock::new().now().unix_timestamp().max(0) as u64,
        ),
        "create TOTP for fresh challenge",
    ));
    flow.track_totp(id, &current_code);
    let current_session = mfa(
        &runtime,
        &router,
        &mut keys,
        "totp",
        &challenge,
        &current_code,
    );
    assert_eq!(
        runtime.block_on(me(&router, &current_session)).status,
        StatusCode::OK
    );
    let current = ok(
        flow.sessions.find_session(&current_session, flow.policy),
        "read new session",
    )
    .expect("new Redis session");
    assert_eq!(current.identity.principal, enrolled.principal);
    assert_eq!(current.identity.auth_generation, after.generation);
    let challenge = login(&runtime, &router, &mut keys, &email, &new_password);
    let recovered = mfa(
        &runtime,
        &router,
        &mut keys,
        "recovery",
        &challenge,
        &enrolled.recovery_codes[1],
    );
    assert_eq!(
        runtime.block_on(me(&router, &recovered)).status,
        StatusCode::OK
    );
    let consumed = flow.snapshot(id);
    assert_eq!(consumed.remaining, RECOVERY_CODE_COUNT - 2);
    assert!(consumed.consumed(0) && consumed.consumed(1));
    assert_eq!(consumed.revision, after.revision + 1);
    assert_eq!(consumed.generation, after.generation);

    let replay = runtime.block_on(post(
        &router,
        COMPLETE,
        &[("token", &token), ("new_password", &new_password)],
    ));
    replay.error(StatusCode::BAD_REQUEST, "password_reset_rejected");
    replay.reset_public(&[&email, &token, &new_password]);
    let final_state = flow.snapshot(id);
    assert!(
        consumed.same_preserved_identity(&final_state),
        "replay changed identity"
    );
    assert!(
        consumed.hash.as_str() == final_state.hash.as_str(),
        "replay changed password"
    );
    assert_eq!(final_state.revision, consumed.revision);
    assert_eq!(final_state.generation, consumed.generation);
    flow.assert_audit(id, after.revision, after.generation);
    assert_eq!(keys.fields(rates::REQUEST_GLOBAL)["count"], "1");
    assert_eq!(keys.fields(&email_rate_key)["count"], "1");
    assert_eq!(keys.fields(rates::COMPLETION_GLOBAL)["count"], "2");
    assert_eq!(keys.fields(&completion_key)["count"], "2");
    assert_eq!(capture.calls.load(SeqCst), 1);

    stop.request();
    runtime.block_on(async {
        tokio::time::timeout(WATCHDOG, consumer)
            .await
            .expect("consumer drainage watchdog")
            .expect("consumer task joined")
            .expect("consumer stopped cleanly");
    });
    drop(runtime);
    drop(router);
    drop(owner);
}
