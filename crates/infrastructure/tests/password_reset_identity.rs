#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
mod password_reset_identity_doubles;
mod password_reset_identity_support;

use application::identity::password_reset::{ResetCompletion, ResetRequestAccepted};
use application::identity::{SecretProtector, SessionStore};
use application::ApplicationError;
use domain::clock::Clock;
use domain::crypto::{TotpProvider, RECOVERY_CODE_COUNT};
use domain::identity::Role;
use infrastructure::{SystemClock, TotpRsProvider};
use uuid::Uuid;
use zeroize::Zeroizing;

use password_reset_identity_support::{ok, Acceptance};

#[test]
fn real_password_reset_revokes_old_redis_credentials_and_preserves_mfa() {
    let mut flow = Acceptance::new();
    let owner = flow.owner_session();
    let email = format!("reset-{}@example.test", Uuid::new_v4().simple());
    flow.track_email(&email);
    let old_password = Zeroizing::new("initial password for isolated acceptance".to_string());
    let new_password = Zeroizing::new("changed password for isolated acceptance".to_string());
    let enrolled = ok(
        flow.identity
            .create_user(&owner, &email, &old_password, Role::Litigator),
        "enroll target with real password and MFA adapters",
    );
    let id = enrolled.principal.id;
    flow.assign_case(id);
    let initial_challenge = flow.challenge(&email, &old_password);
    let initial_session = ok(
        flow.identity
            .complete_recovery(&initial_challenge, &enrolled.recovery_codes[0]),
        "authenticate before reset using recovery",
    );
    let old_session = flow.session(initial_session);
    assert_eq!(
        ok(
            flow.identity.authenticate(&old_session),
            "admit old session"
        ),
        enrolled.principal
    );
    let old_totp_challenge = flow.challenge(&email, &old_password);
    let old_recovery_challenge = flow.challenge(&email, &old_password);
    let before = flow.snapshot(id);
    assert_eq!(before.generation, 0);
    assert_eq!(before.remaining, RECOVERY_CODE_COUNT - 1);
    assert!(before.consumed(0));
    assert!(!before.consumed(1));

    assert_eq!(
        ok(
            flow.reset
                .request(&format!("  {}  ", email.to_ascii_uppercase())),
            "request internal reset"
        ),
        ResetRequestAccepted
    );
    let envelope = flow.delivery.take();
    assert_eq!(envelope.email, email);
    assert_eq!(envelope.token.len(), 32);
    assert!(envelope.expires_at > SystemClock::new().now());
    flow.assert_reset_digest(envelope.token.as_ref());
    assert_eq!(
        ok(
            flow.reset.complete(envelope.token.as_ref(), &new_password),
            "complete internal reset"
        ),
        ResetCompletion::Changed
    );
    let after = flow.snapshot(id);
    assert!(
        before.same_preserved_identity(&after),
        "reset changed preserved identity material"
    );
    assert!(
        before.hash.as_str() != after.hash.as_str(),
        "password hash did not change"
    );
    assert!(after.hash.starts_with("$argon2id$v=19$"));
    assert!(after.hash.contains("m=262144,t=3,p=1"));
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.generation, before.generation + 1);
    assert!(after.consumed(0));
    assert!(!after.consumed(1));

    // Redis retains the old credential; durable generation denies its admission.
    let cached = ok(
        flow.sessions.find_session(&old_session, flow.policy),
        "read retained old Redis session",
    )
    .expect("old Redis session remains alive");
    assert_eq!(cached.identity.auth_generation, before.generation);
    assert!(cached.absolute_expires_at_unix_ms > cached.server_now_unix_ms);
    assert!(matches!(
        flow.identity.authenticate(&old_session),
        Err(ApplicationError::InvalidSession)
    ));
    let secret = ok(
        flow.secrets.expose(id, &after.protected_totp),
        "expose existing TOTP secret",
    );
    let stale_code = Zeroizing::new(ok(
        TotpRsProvider::new().current_code(
            &secret,
            SystemClock::new().now().unix_timestamp().max(0) as u64,
        ),
        "generate real TOTP",
    ));
    flow.track_totp(id, &stale_code);
    assert!(matches!(
        flow.identity
            .complete_totp(&old_totp_challenge, &stale_code),
        Err(ApplicationError::MfaRejected)
    ));
    assert!(matches!(
        flow.identity
            .complete_recovery(&old_recovery_challenge, &enrolled.recovery_codes[1],),
        Err(ApplicationError::MfaRejected)
    ));
    let denied = flow.snapshot(id);
    assert!(
        after.same_preserved_identity(&denied),
        "old challenges altered MFA material"
    );
    assert_eq!(denied.revision, after.revision);
    assert_eq!(denied.generation, after.generation);
    assert!(matches!(
        flow.identity.start_login(&email, &old_password),
        Err(ApplicationError::InvalidCredentials)
    ));

    let new_totp_challenge = flow.challenge(&email, &new_password);
    let current_code = Zeroizing::new(ok(
        TotpRsProvider::new().current_code(
            &secret,
            SystemClock::new().now().unix_timestamp().max(0) as u64,
        ),
        "generate TOTP after new password login",
    ));
    flow.track_totp(id, &current_code);
    let totp_session = ok(
        flow.identity
            .complete_totp(&new_totp_challenge, &current_code),
        "existing TOTP completes new password login",
    );
    assert_eq!(totp_session.principal, enrolled.principal);
    let totp_token = flow.session(totp_session);
    assert_eq!(
        ok(
            flow.identity.authenticate(&totp_token),
            "admit new TOTP session"
        ),
        enrolled.principal
    );
    let current = ok(
        flow.sessions.find_session(&totp_token, flow.policy),
        "read new session",
    )
    .expect("new session exists");
    assert_eq!(current.identity.auth_generation, after.generation);

    let new_recovery_challenge = flow.challenge(&email, &new_password);
    let recovered = ok(
        flow.identity
            .complete_recovery(&new_recovery_challenge, &enrolled.recovery_codes[1]),
        "existing recovery completes new password login",
    );
    assert_eq!(recovered.principal, enrolled.principal);
    let recovered_token = flow.session(recovered);
    assert_eq!(
        ok(
            flow.identity.authenticate(&recovered_token),
            "admit new recovery session"
        ),
        enrolled.principal
    );
    let consumed = flow.snapshot(id);
    assert_eq!(consumed.remaining, RECOVERY_CODE_COUNT - 2);
    assert!(consumed.consumed(0));
    assert!(consumed.consumed(1));
    assert_eq!(consumed.generation, after.generation);
    assert_eq!(consumed.revision, after.revision + 1);
    assert!(
        consumed.protected_totp == after.protected_totp,
        "recovery replaced TOTP"
    );
    assert_eq!(consumed.principal, after.principal);
    assert_eq!(consumed.memberships, after.memberships);

    assert_eq!(
        ok(
            flow.reset.complete(envelope.token.as_ref(), &new_password),
            "reject reset replay"
        ),
        ResetCompletion::Rejected
    );
    let replay = flow.snapshot(id);
    assert!(
        consumed.same_preserved_identity(&replay),
        "reset replay changed identity"
    );
    assert!(
        consumed.hash.as_str() == replay.hash.as_str(),
        "reset replay changed password"
    );
    assert_eq!(replay.revision, consumed.revision);
    assert_eq!(replay.generation, consumed.generation);
    flow.assert_audit(id, after.revision, after.generation);
}
