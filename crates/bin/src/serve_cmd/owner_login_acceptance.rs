mod evidence;
mod material;
mod support;

use crate::{
    password_reset_composition_support::router_with_authentication,
    password_reset_identity_support::{ok, Acceptance},
    password_reset_runtime_support::Fixture,
    serve_identity_composition,
    serve_password_reset_http_acceptance_support::{login, me, mfa, post},
};
use application::{
    credential_trust::CredentialTrustExpectation,
    identity::{
        certificate_login::SessionAuthentication, SecretProtector, SessionPolicy, SessionStore,
    },
};
use axum::http::StatusCode;
use domain::{clock::Clock, crypto::TotpProvider, identity::Role};
use infrastructure::{SystemClock, TotpRsProvider};
use material::Material;
use serde_json::json;
use std::{num::NonZeroUsize, sync::Arc};
use support::*;
use uuid::Uuid;
use web::{AuthenticationHttp, HttpLimits, HttpWorkBudget};
use zeroize::Zeroizing;

#[test]
fn composed_owner_login_requires_real_rsa_mfa_and_current_binding_authority() {
    require_backends();
    let mut keys = Fixture::new();
    let mut flow = Acceptance::new();
    let seed = flow.owner_session();
    let email = format!("owner-login-{}@example.test", Uuid::new_v4().simple());
    flow.track_email(&email);
    let password = Zeroizing::new("native owner login password".to_owned());
    let enrolled = ok(
        flow.identity
            .create_user(&seed, &email, &password, Role::Owner),
        "enroll actual login Owner",
    );
    let principal = enrolled.principal.clone();
    let binding = Uuid::new_v4();
    own_login_keys(&mut keys, principal.id, binding);
    let material = Material::new();
    let first_trust = material.publish(&flow.db.admin_url, CredentialTrustExpectation::Absent);
    let policy = ok(
        SessionPolicy::new(3600, None),
        "explicit native session policy",
    );
    let components = serve_identity_composition::open(
        &flow.db.runtime_url,
        &keys.url,
        Zeroizing::new(vec![42; 32]),
        policy,
        Some(&settings()),
    )
    .unwrap_or_else(|_| panic!("open actual certificate-aware identity composition"));
    let certificate_login = components
        .certificate_login
        .expect("enabled certificate workflow");
    assert!(
        Arc::as_ptr(&components.identity) as *const ()
            == Arc::as_ptr(&certificate_login) as *const (),
        "password and certificate workflow views must share one identity instance"
    );
    let owner_service =
        super::owner_certificates::open(&flow.db.runtime_url, components.identity.clone())
            .unwrap_or_else(|_| panic!("compose real authenticated Owner binding service"));
    let budget = HttpWorkBudget::new(NonZeroUsize::new(1).unwrap());
    let router = router_with_authentication(
        components.identity,
        owner_service,
        AuthenticationHttp {
            password_reset: None,
            certificate_login: Some(certificate_login),
        },
        HttpLimits {
            max_requests: NonZeroUsize::new(4).unwrap(),
            max_blocking_operations: NonZeroUsize::new(1).unwrap(),
        },
        budget,
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let password_challenge = login(&runtime, &router, &mut keys, &email, &password);
    let password_session = mfa(
        &runtime,
        &router,
        &mut keys,
        "recovery",
        &password_challenge,
        &enrolled.recovery_codes[0],
    );
    let password_state = ok(
        flow.sessions.find_session(&password_session, policy),
        "read password origin",
    )
    .expect("password session exists");
    assert!(matches!(
        password_state.authentication,
        SessionAuthentication::Password
    ));
    let receipt = register(&runtime, &router, &material, &password_session, binding);
    assert_eq!(receipt["owner_id"], principal.id.to_string());

    let bad = start(&runtime, &router, &mut keys, &material, &principal, binding);
    let mut wrong = bad.signature.clone();
    wrong[0] ^= 1;
    proof_rejected(&runtime, &router, &bad, &wrong);
    absent_capture(&mut keys, &bad);
    proof_rejected(&runtime, &router, &bad, &bad.signature);
    let accepted = start(&runtime, &router, &mut keys, &material, &principal, binding);
    runtime
        .block_on(me(&router, &accepted.token))
        .error(StatusCode::UNAUTHORIZED, "invalid_session");
    let challenge = proof(&runtime, &router, &mut keys, &accepted);
    absent_capture(&mut keys, &accepted);
    proof_rejected(&runtime, &router, &accepted, &accepted.signature);
    runtime
        .block_on(me(&router, &challenge))
        .error(StatusCode::UNAUTHORIZED, "invalid_session");
    let current = flow.snapshot(principal.id);
    let secret = ok(
        flow.secrets.expose(principal.id, &current.protected_totp),
        "expose actual enrollment TOTP",
    );
    let code = Zeroizing::new(ok(
        TotpRsProvider::new().current_code(
            &secret,
            SystemClock::new().now().unix_timestamp().max(0) as u64,
        ),
        "calculate actual TOTP",
    ));
    flow.track_totp(principal.id, &code);
    let first_session = mfa(&runtime, &router, &mut keys, "totp", &challenge, &code);
    evidence::certificate_origin(
        &flow,
        policy,
        &first_session,
        &principal,
        &first_trust,
        &receipt,
        &accepted,
    );
    evidence::admitted(&runtime, &router, &first_session, binding, &receipt);

    let pending_capture = start(&runtime, &router, &mut keys, &material, &principal, binding);
    let pending_mfa = proof(&runtime, &router, &mut keys, &pending_capture);
    let pending_first = start(&runtime, &router, &mut keys, &material, &principal, binding);
    let before_rotation = flow.snapshot(principal.id);
    let next_trust = material.rotate(&flow.db.admin_url, &first_trust);
    proof_rejected(&runtime, &router, &pending_first, &pending_first.signature);
    absent_capture(&mut keys, &pending_first);
    evidence::rejected_mfa(&runtime, &router, &pending_mfa, &enrolled.recovery_codes[1]);
    evidence::denied(&runtime, &router, &first_session, binding);
    evidence::admitted(&runtime, &router, &password_session, binding, &receipt);
    let after_rotation = flow.snapshot(principal.id);
    assert!(
        before_rotation.same_preserved_identity(&after_rotation),
        "rotation rejection changed account/MFA"
    );
    assert!(
        before_rotation.hash.as_str() == after_rotation.hash.as_str(),
        "rotation rejection changed password"
    );
    assert_eq!(before_rotation.revision, after_rotation.revision);
    assert_eq!(before_rotation.generation, after_rotation.generation);

    let renewed = start(&runtime, &router, &mut keys, &material, &principal, binding);
    let renewed_mfa = proof(&runtime, &router, &mut keys, &renewed);
    let second_session = mfa(
        &runtime,
        &router,
        &mut keys,
        "recovery",
        &renewed_mfa,
        &enrolled.recovery_codes[1],
    );
    evidence::certificate_origin(
        &flow,
        policy,
        &second_session,
        &principal,
        &next_trust,
        &receipt,
        &renewed,
    );
    evidence::admitted(&runtime, &router, &second_session, binding, &receipt);
    let pending_capture = start(&runtime, &router, &mut keys, &material, &principal, binding);
    let pending_mfa = proof(&runtime, &router, &mut keys, &pending_capture);
    let pending_first = start(&runtime, &router, &mut keys, &material, &principal, binding);
    let before_withdrawal = flow.snapshot(principal.id);
    let terminal = public_json(
        &runtime,
        &router,
        "POST",
        &format!("{}/withdraw", binding_route(binding)),
        &password_session,
        Some(json!({"expected_revision": 1})),
        StatusCode::OK,
    );
    assert_eq!(terminal["registration"], receipt["registration"]);
    assert_eq!(terminal["revision"], 2);
    assert!(!terminal["withdrawal"].is_null());
    proof_rejected(&runtime, &router, &pending_first, &pending_first.signature);
    absent_capture(&mut keys, &pending_first);
    evidence::rejected_mfa(&runtime, &router, &pending_mfa, &enrolled.recovery_codes[2]);
    evidence::denied(&runtime, &router, &second_session, binding);
    evidence::admitted(&runtime, &router, &password_session, binding, &terminal);
    runtime
        .block_on(post(
            &router,
            START,
            &[
                ("owner_id", &principal.id.to_string()),
                ("binding_id", &binding.to_string()),
            ],
        ))
        .error(StatusCode::UNAUTHORIZED, "invalid_credentials");
    let after_withdrawal = flow.snapshot(principal.id);
    assert!(
        before_withdrawal.same_preserved_identity(&after_withdrawal),
        "withdrawal rejection changed account/MFA"
    );
    assert!(
        before_withdrawal.hash.as_str() == after_withdrawal.hash.as_str(),
        "withdrawal rejection changed password"
    );
    assert_eq!(before_withdrawal.revision, after_withdrawal.revision);
    assert_eq!(before_withdrawal.generation, after_withdrawal.generation);
    assert!(public_json(
        &runtime,
        &router,
        "GET",
        "/api/v1/auth/certificate-bindings/current",
        &password_session,
        None,
        StatusCode::OK
    )
    .is_null());
    let password_state = ok(
        flow.sessions.find_session(&password_session, policy),
        "read surviving password session",
    )
    .expect("independent password session retained");
    assert!(matches!(
        password_state.authentication,
        SessionAuthentication::Password
    ));
    evidence::audit(&mut flow, binding);
    drop(runtime);
    drop(router);
}
