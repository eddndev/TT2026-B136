use crate::support::*;

#[test]
fn a_service_without_certificate_authority_rejects_its_mfa_and_bearer_but_keeps_passwords() {
    let env = Env::new();
    let guarded = env.service(true, SessionPolicy::default());
    let ordinary = env.service(false, SessionPolicy::default());
    let challenge = env.proof(&guarded);
    assert!(matches!(
        ordinary.complete_totp(&challenge.challenge_token, "123456"),
        Err(ApplicationError::MfaRejected)
    ));
    env.read(|s| {
        assert_eq!(s.factor_calls, 0);
        assert_eq!(s.secret_exposures, 0);
    });
    let certificate = env.certificate_session(&guarded);
    assert!(matches!(
        ordinary.authenticate(&certificate.access_token),
        Err(ApplicationError::InvalidSession)
    ));
    let password = env.password_session(&ordinary);
    assert_eq!(
        ordinary.authenticate(&password.access_token).unwrap(),
        password.principal
    );
    env.read(|s| {
        assert_eq!(
            s.sessions[&password.access_token].1.authentication,
            SessionAuthentication::Password
        )
    });
}

#[test]
fn withdrawn_rotated_changed_or_expired_authority_blocks_mfa_before_secret_exposure() {
    for change in [
        Change::Trust,
        Change::Withdraw,
        Change::Principal,
        Change::Generation,
        Change::Advance(1300),
    ] {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let challenge = env.proof(&service);
        env.edit(|s| s.change(change));
        assert!(
            matches!(
                service.complete_totp(&challenge.challenge_token, "123456"),
                Err(ApplicationError::MfaRejected)
            ),
            "{change:?}"
        );
        env.read(|s| {
            assert_eq!(s.secret_exposures, 0);
            assert_eq!(s.factor_calls, 0);
            assert!(s.mfa.is_empty());
            assert_eq!(s.sessions_created, 0);
        });
    }
}

#[test]
fn authority_is_checked_again_after_factor_work_and_after_grant_creation() {
    for after_creation in [false, true] {
        for change in [
            Change::Trust,
            Change::Withdraw,
            Change::Principal,
            Change::Generation,
            Change::Advance(2000),
        ] {
            let env = Env::new();
            let service = env.service(true, SessionPolicy::default());
            let challenge = env.proof(&service);
            env.edit(|s| {
                if after_creation {
                    s.after_session_create = Some(change);
                } else {
                    s.after_factor = Some(change);
                }
            });
            assert!(
                matches!(
                    service.complete_totp(&challenge.challenge_token, "123456"),
                    Err(ApplicationError::MfaRejected)
                ),
                "after creation {after_creation}, {change:?}"
            );
            env.read(|s| {
                assert_eq!(s.factor_calls, 1);
                assert_eq!(s.sessions_created, usize::from(after_creation));
                assert_eq!(s.revocations, usize::from(after_creation));
                assert!(s.sessions.is_empty());
                assert!(s.mfa.is_empty());
            });
        }
    }
}

#[test]
fn recovery_revision_increment_is_legitimate_but_a_concurrent_generation_change_is_not() {
    for changed in [false, true] {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let challenge = env.proof(&service);
        env.edit(|s| s.after_factor = changed.then_some(Change::Generation));
        let result = service.complete_recovery(&challenge.challenge_token, "RECOVERY-0");
        if changed {
            assert!(matches!(result, Err(ApplicationError::MfaRejected)));
        } else {
            let session = result.unwrap();
            assert_eq!(
                service.authenticate(&session.access_token).unwrap(),
                session.principal
            );
            assert_eq!(session.session.absolute_expires_at_unix_ms, 2_000_000);
        }
        env.read(|s| {
            assert_eq!(s.user.revision, 10);
            assert_eq!(s.user.auth_generation, if changed { 5 } else { 4 });
            assert_eq!(s.sessions_created, usize::from(!changed));
        });
        let next = env.proof(&service);
        assert!(matches!(
            service.complete_recovery(&next.challenge_token, "RECOVERY-0"),
            Err(ApplicationError::MfaRejected)
        ));
    }
}

#[test]
fn near_expiry_caps_each_challenge_and_the_session_without_extending_authority() {
    let env = Env::new();
    let service = env.service(true, SessionPolicy::default());
    env.edit(|s| s.now = 1850);
    let challenge = env.begin(&service);
    assert_eq!(challenge.expires_in_seconds, 150);
    assert_eq!(challenge.statement.expires_at_unix_seconds(), 2000);
    env.edit(|s| s.now = 1900);
    let mfa = service
        .prove_certificate_login(&challenge.challenge_token, &[5; 384])
        .unwrap();
    assert_eq!(mfa.expires_in_seconds, 100);
    env.edit(|s| s.now = 1999);
    let result = service
        .complete_totp(&mfa.challenge_token, "123456")
        .unwrap();
    assert_eq!(result.expires_in_seconds, 1);
    assert_eq!(result.session.absolute_expires_at_unix_ms, 2_000_000);
}

#[test]
fn audit_failures_remove_new_mfa_or_bearer_without_issuing_a_replacement() {
    for after_proof in [false, true] {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let token = if after_proof {
            env.proof(&service).challenge_token
        } else {
            env.begin(&service).challenge_token
        };
        env.edit(|s| s.fail_audit = true);
        if after_proof {
            assert!(service.complete_totp(&token, "123456").is_err());
        } else {
            assert!(service.prove_certificate_login(&token, &[5; 384]).is_err());
        }
        env.read(|s| {
            assert!(s.mfa.is_empty());
            assert!(s.sessions.is_empty());
            assert_eq!(s.sessions_created, usize::from(after_proof));
            assert_eq!(s.revocations, usize::from(after_proof));
            assert_eq!(s.verifier_calls, 1);
        });
    }
}
