use std::sync::Arc;

use crate::{
    serve_identity_composition::{compose, IdentityComponents},
    support::*,
};
use application::identity::IdentityWorkflow;

fn components(env: &Env, enabled: bool) -> IdentityComponents {
    let shared = Arc::new(env.clone());
    let identity = IdentityPorts {
        users: shared.clone(),
        sessions: shared.clone(),
        passwords: shared.clone(),
        totp: shared.clone(),
        recovery: shared.clone(),
        secrets: shared.clone(),
        clock: shared.clone(),
        audit_log: Box::new(env.clone()),
    };
    let certificate = enabled.then(|| CertificateLoginPorts {
        authority: shared.clone(),
        runtime: shared.clone(),
        verifier: shared,
        hasher: Arc::new(Hasher),
    });
    compose(
        identity,
        SessionPolicy::new(3600, Some(120)).unwrap(),
        certificate,
    )
}

fn proof(components: &IdentityComponents) -> LoginChallenge {
    let certificate = components.certificate_login.as_ref().unwrap();
    let challenge = certificate
        .start_certificate_login(user().id, binding())
        .unwrap();
    certificate
        .prove_certificate_login(&challenge.challenge_token, &[5; 384])
        .unwrap()
}

fn password_session(identity: &dyn IdentityWorkflow) -> SessionResult {
    let challenge = identity
        .start_login("owner@example.test", "password")
        .unwrap();
    identity
        .complete_totp(&challenge.challenge_token, "123456")
        .unwrap()
}

#[test]
fn disabled_composition_retains_password_mfa_without_owner_port_calls() {
    let env = Env::new();
    let components = components(&env, false);
    assert!(components.certificate_login.is_none());
    let session = password_session(components.identity.as_ref());
    assert_eq!(
        session.session.policy,
        SessionPolicy::new(3600, Some(120)).unwrap()
    );
    assert_eq!(
        components
            .identity
            .authenticate(&session.access_token)
            .unwrap(),
        session.principal
    );
    components
        .identity
        .record_activity(&session.access_token)
        .unwrap();
    env.read(|s| {
        assert_eq!(s.authority_reads, 0);
        assert_eq!(s.start_admissions, 0);
        assert_eq!(s.proof_admissions, 0);
        assert_eq!(s.verifier_calls, 0);
        assert_eq!(
            s.sessions[&session.access_token].1.authentication,
            SessionAuthentication::Password
        );
    });
}

#[test]
fn enabled_views_share_one_allocation_and_preserve_origin_through_mfa_and_admission() {
    for recovery in [false, true] {
        let env = Env::new();
        let components = components(&env, true);
        assert_eq!(
            Arc::as_ptr(&components.identity) as *const (),
            Arc::as_ptr(components.certificate_login.as_ref().unwrap()) as *const ()
        );
        let challenge = proof(&components);
        env.read(|s| {
            assert_eq!(s.sessions_created, 0);
            assert_eq!(s.mfa_created, 1);
            assert_eq!(s.verifier_calls, 1);
            assert!(matches!(
                &s.mfa[&challenge.challenge_token].0,
                MfaChallenge::Certificate(_)
            ));
        });
        let identity = &components.identity;
        let session = if recovery {
            identity.complete_recovery(&challenge.challenge_token, "RECOVERY-0")
        } else {
            identity.complete_totp(&challenge.challenge_token, "123456")
        }
        .unwrap();
        env.read(|s| {
            let SessionAuthentication::Certificate(origin) =
                &s.sessions[&session.access_token].1.authentication
            else {
                panic!("certificate origin was lost by identity composition")
            };
            assert_eq!(origin.binding_id, binding());
            assert_eq!(origin.crl_digest, context().trust.inspection.crl_digest);
            assert_eq!(origin.auth_generation, user().auth_generation);
        });
        assert_eq!(session.session.absolute_expires_at_unix_ms, 2_000_000);
        assert_eq!(session.session.idle_expires_at_unix_ms, Some(1_120_000));
        assert_eq!(
            identity.authenticate(&session.access_token).unwrap(),
            session.principal
        );
        env.edit(|s| s.now = 1050);
        let status = identity.session_status(&session.access_token).unwrap();
        assert_eq!(status.idle_expires_at_unix_ms, Some(1_120_000));
        let activity = identity.record_activity(&session.access_token).unwrap();
        assert_eq!(activity.idle_expires_at_unix_ms, Some(1_170_000));
        assert_eq!(activity.absolute_expires_at_unix_ms, 2_000_000);
        let password = password_session(identity.as_ref());
        env.edit(|s| {
            s.change(if recovery {
                Change::Withdraw
            } else {
                Change::Trust
            })
        });
        assert!(matches!(
            identity.authenticate(&session.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(matches!(
            identity.session_status(&session.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(matches!(
            identity.record_activity(&session.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert_eq!(
            identity.authenticate(&password.access_token).unwrap(),
            password.principal
        );
        identity.record_activity(&password.access_token).unwrap();
    }
}

#[test]
fn disabling_on_recomposition_rejects_existing_certificate_mfa_and_sessions_without_downgrade() {
    let env = Env::new();
    let enabled = components(&env, true);
    let pending = proof(&enabled);
    let completed = proof(&enabled);
    let certificate = enabled
        .identity
        .complete_totp(&completed.challenge_token, "123456")
        .unwrap();
    let password = password_session(enabled.identity.as_ref());
    let counts = env.read(|s| {
        (
            s.factor_calls,
            s.sessions_created,
            s.authority_reads,
            s.touches,
        )
    });
    let saved = env.read(|s| s.sessions[&certificate.access_token].clone());
    let disabled = components(&env, false);
    assert!(disabled.certificate_login.is_none());
    assert!(matches!(
        disabled
            .identity
            .complete_totp(&pending.challenge_token, "123456"),
        Err(ApplicationError::MfaRejected)
    ));
    assert!(matches!(
        disabled.identity.authenticate(&certificate.access_token),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        disabled.identity.session_status(&certificate.access_token),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        disabled.identity.record_activity(&certificate.access_token),
        Err(ApplicationError::InvalidSession)
    ));
    env.read(|s| {
        assert_eq!(
            (
                s.factor_calls,
                s.sessions_created,
                s.authority_reads,
                s.touches
            ),
            counts
        );
        assert_eq!(s.sessions[&certificate.access_token], saved);
        assert!(!s.mfa.contains_key(&pending.challenge_token));
    });
    assert_eq!(
        disabled
            .identity
            .authenticate(&password.access_token)
            .unwrap(),
        password.principal
    );
    disabled
        .identity
        .record_activity(&password.access_token)
        .unwrap();
}
