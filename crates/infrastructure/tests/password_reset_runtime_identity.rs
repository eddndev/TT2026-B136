#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "password_reset_cases/password_reset_identity_doubles.rs"]
mod password_reset_identity_doubles;
#[allow(dead_code)]
#[path = "password_reset_cases/password_reset_identity_support.rs"]
mod password_reset_identity_support;
#[allow(dead_code, unused_imports)]
#[path = "password_reset_cases/password_reset_runtime_support.rs"]
mod password_reset_runtime_support;

use std::sync::Arc;
use std::time::Duration;

use application::identity::password_reset::{
    PasswordResetPorts, PasswordResetService, ResetCompletion, ResetPolicy, ResetRequestAccepted,
};
use application::identity::{SecretProtector, SessionStore};
use application::ApplicationError;
use domain::clock::Clock;
use domain::crypto::{DocumentHasher, TotpProvider, RECOVERY_CODE_COUNT};
use domain::identity::Role;
use infrastructure::identity::{
    PasswordResetRatePolicy, PostgresPasswordResetRepository, RandomResetTokenSource,
    RedisPasswordResetLimiter,
};
use infrastructure::{Argon2idHasher, RingSha256Hasher, SystemClock, TotpRsProvider};
use redis::Commands;
use zeroize::Zeroizing;

use password_reset_identity_support::{ok, Acceptance};
use password_reset_runtime_support::{
    completion_key, limit, Fixture as RedisFixture, COMPLETION_GLOBAL, REQUEST_GLOBAL,
};

fn attach_runtime_adapters(flow: &mut Acceptance, redis: &RedisFixture) {
    let quotas =
        PasswordResetRatePolicy::new(limit(2, 120), limit(1, 60), limit(3, 120), limit(2, 60));
    // Replace the helper's reset service before its first invocation. Identity,
    // protected MFA, audited storage and local delivery retain the shared fixture.
    flow.reset = PasswordResetService::new(
        PasswordResetPorts {
            repository: Arc::new(ok(
                PostgresPasswordResetRepository::open(&flow.db.runtime_url),
                "open real reset repository",
            )),
            delivery: flow.delivery.clone(),
            limiter: Arc::new(ok(
                RedisPasswordResetLimiter::connect(
                    &redis.url,
                    quotas,
                    Duration::from_secs(1),
                    Duration::from_secs(1),
                ),
                "configure real reset limiter",
            )),
            tokens: Arc::new(RandomResetTokenSource),
            digests: Arc::new(RingSha256Hasher),
            passwords: Arc::new(Argon2idHasher::new()),
        },
        ok(ResetPolicy::new(300, 2), "explicit reset capability policy"),
    );
}

#[test]
fn operating_system_tokens_and_redis_budgets_complete_an_audited_password_and_mfa_flow() {
    let mut budgets = RedisFixture::new();
    let mut flow = Acceptance::new();
    attach_runtime_adapters(&mut flow, &budgets);
    let (email, request_key) = budgets.email();
    flow.track_email(&email);
    let owner = flow.owner_session();
    let old_password = Zeroizing::new("initial password for runtime reset acceptance".to_string());
    let new_password = Zeroizing::new("changed password for runtime reset acceptance".to_string());
    let enrolled = ok(
        flow.identity
            .create_user(&owner, &email, &old_password, Role::Litigator),
        "enroll with real Argon2id and MFA",
    );
    let id = enrolled.principal.id;
    flow.assign_case(id);
    let initial_challenge = flow.challenge(&email, &old_password);
    let initial_session = ok(
        flow.identity
            .complete_recovery(&initial_challenge, &enrolled.recovery_codes[0]),
        "authenticate before reset",
    );
    let old_session = flow.session(initial_session);
    let stale_challenge = flow.challenge(&email, &old_password);
    let before = flow.snapshot(id);
    assert_eq!(before.remaining, RECOVERY_CODE_COUNT - 1);
    assert!(before.consumed(0) && !before.consumed(1));

    assert_eq!(
        ok(
            flow.reset
                .request(&format!(" {} ", email.to_ascii_uppercase())),
            "request with operating-system entropy"
        ),
        ResetRequestAccepted
    );
    let requested = budgets.snapshot();
    assert_eq!(budgets.fields(REQUEST_GLOBAL)["count"], "1");
    assert_eq!(budgets.fields(&request_key)["count"], "1");
    assert_eq!(budgets.fields(REQUEST_GLOBAL)["window_ms"], "120000");
    assert_eq!(budgets.fields(&request_key)["window_ms"], "60000");
    // Delivery still holds the first envelope and rejects any second delivery.
    assert_eq!(
        ok(
            flow.reset.request(&email),
            "request denied by the email budget"
        ),
        ResetRequestAccepted
    );
    assert!(
        budgets.snapshot() == requested,
        "request rejection changed quota or expiry"
    );
    let envelope = flow.delivery.take();
    assert_eq!(envelope.email, email);
    assert_eq!(envelope.token.len(), 32);
    assert!(envelope.expires_at > SystemClock::new().now());
    flow.assert_reset_digest(envelope.token.as_ref());
    let (issued, pending, policy_seconds): (i64, i64, i64) = {
        let row = ok(flow.db.admin.query_one(
            "SELECT count(*), count(*) FILTER (WHERE consumed_at IS NULL AND cancelled_at IS NULL),
             min(extract(epoch FROM expires_at-issued_at))::bigint FROM password_reset_capabilities",
            &[]), "read unique pending reset issuance");
        (row.get(0), row.get(1), row.get(2))
    };
    assert_eq!((issued, pending, policy_seconds), (1, 1, 300));

    let mut input = Zeroizing::new(b"qadra:password-reset:v1\0".to_vec());
    input.extend_from_slice(envelope.token.as_ref());
    let completion_key = completion_key(RingSha256Hasher.hash_bytes(&input));
    assert!(
        !budgets
            .connection
            .exists::<_, bool>(&completion_key)
            .unwrap(),
        "completion fixture refuses to claim an existing digest budget"
    );
    budgets.track(completion_key.clone());
    assert_eq!(
        ok(
            flow.reset.complete(envelope.token.as_ref(), &new_password),
            "complete with SQL and Argon2id"
        ),
        ResetCompletion::Changed
    );
    assert_eq!(budgets.fields(COMPLETION_GLOBAL)["count"], "1");
    assert_eq!(budgets.fields(&completion_key)["count"], "1");
    let after = flow.snapshot(id);
    assert!(
        before.same_preserved_identity(&after),
        "reset changed preserved identity or MFA"
    );
    assert!(
        before.hash.as_str() != after.hash.as_str(),
        "password hash did not change"
    );
    assert!(after.hash.starts_with("$argon2id$v=19$") && after.hash.contains("m=262144,t=3,p=1"));
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.generation, before.generation + 1);
    let consumed: i64 = ok(
        flow.db.admin.query_one(
            "SELECT count(*) FROM password_reset_capabilities WHERE consumed_at IS NOT NULL
         AND cancelled_at IS NULL AND consumed_revision=$1 AND consumed_generation=$2",
            &[&(after.revision as i64), &(after.generation as i64)],
        ),
        "verify consumed capability receipt",
    )
    .get(0);
    assert_eq!(consumed, 1);
    flow.assert_audit(id, after.revision, after.generation);

    let cached = ok(
        flow.sessions.find_session(&old_session, flow.policy),
        "read old Redis credential",
    )
    .expect("old session remains in Redis");
    assert_eq!(cached.identity.auth_generation, before.generation);
    assert!(matches!(
        flow.identity.authenticate(&old_session),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        flow.identity
            .complete_recovery(&stale_challenge, &enrolled.recovery_codes[1]),
        Err(ApplicationError::MfaRejected)
    ));
    assert!(matches!(
        flow.identity.start_login(&email, &old_password),
        Err(ApplicationError::InvalidCredentials)
    ));
    let denied = flow.snapshot(id);
    assert!(
        after.same_preserved_identity(&denied),
        "stale credential changed MFA"
    );
    assert_eq!(denied.revision, after.revision);
    assert_eq!(denied.generation, after.generation);

    let fresh_challenge = flow.challenge(&email, &new_password);
    let secret = ok(
        flow.secrets.expose(id, &after.protected_totp),
        "read unchanged protected TOTP",
    );
    let code = Zeroizing::new(ok(
        TotpRsProvider::new().current_code(
            &secret,
            SystemClock::new().now().unix_timestamp().max(0) as u64,
        ),
        "generate current TOTP",
    ));
    flow.track_totp(id, &code);
    let authenticated = ok(
        flow.identity.complete_totp(&fresh_challenge, &code),
        "new password still requires the existing MFA secret",
    );
    let fresh_session = flow.session(authenticated);
    assert_eq!(
        ok(
            flow.identity.authenticate(&fresh_session),
            "authenticate new MFA session"
        ),
        enrolled.principal
    );
    let current = ok(
        flow.sessions.find_session(&fresh_session, flow.policy),
        "read authenticated generation",
    )
    .expect("new session remains in Redis");
    assert_eq!(current.identity.auth_generation, after.generation);

    assert_eq!(
        ok(
            flow.reset.complete(envelope.token.as_ref(), &new_password),
            "SQL rejects consumed capability within budget"
        ),
        ResetCompletion::Rejected
    );
    assert_eq!(budgets.fields(COMPLETION_GLOBAL)["count"], "2");
    assert_eq!(budgets.fields(&completion_key)["count"], "2");
    let exhausted = budgets.snapshot();
    assert_eq!(
        ok(
            flow.reset.complete(envelope.token.as_ref(), &new_password),
            "Redis rejects exhausted digest budget"
        ),
        ResetCompletion::Rejected
    );
    assert!(
        budgets.snapshot() == exhausted,
        "completion quota rejection changed Redis state"
    );
    let replay = flow.snapshot(id);
    assert!(
        after.same_preserved_identity(&replay),
        "replay changed preserved identity"
    );
    assert!(
        after.hash.as_str() == replay.hash.as_str(),
        "replay changed password"
    );
    assert_eq!(replay.revision, after.revision);
    assert_eq!(replay.generation, after.generation);
    flow.assert_audit(id, after.revision, after.generation);
}
