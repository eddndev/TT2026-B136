use domain::crypto::DocumentHasher;

use crate::support::*;

#[test]
fn changed_crl_at_the_same_revision_rejects_mfa_session_status_and_activity() {
    for entry in 0..4 {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let token = if entry == 0 {
            env.proof(&service).challenge_token
        } else {
            env.certificate_session(&service).access_token
        };
        env.edit(|s| {
            let trust = &mut s.context.as_mut().unwrap().trust;
            trust.inspection.crl_der = b"different-crl-at-same-revision".to_vec();
            trust.inspection.crl_digest = Hasher.hash_bytes(&trust.inspection.crl_der);
            assert_eq!(trust.revision, context().trust.revision);
            assert_eq!(
                trust.inspection.root_fingerprint,
                context().trust.inspection.root_fingerprint
            );
            assert_ne!(
                trust.inspection.crl_digest,
                context().trust.inspection.crl_digest
            );
        });
        let result = match entry {
            0 => service.complete_totp(&token, "123456").map(|_| ()),
            1 => service.authenticate(&token).map(|_| ()),
            2 => service.session_status(&token).map(|_| ()),
            _ => service.record_activity(&token).map(|_| ()),
        };
        if entry == 0 {
            assert!(matches!(result, Err(ApplicationError::MfaRejected)));
        } else {
            assert!(
                matches!(result, Err(ApplicationError::InvalidSession)),
                "entry {entry}"
            );
        }
        env.read(|s| {
            assert_eq!(s.secret_exposures, usize::from(entry != 0));
            assert_eq!(s.sessions_created, usize::from(entry != 0));
            assert_eq!(s.touches, 0);
        });
    }
}

#[test]
fn withdrawn_or_successor_trust_blocks_every_certificate_entry_without_blocking_passwords() {
    for change in [Change::Trust, Change::Withdraw] {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let certificate = env.certificate_session(&service);
        let password = env.password_session(&service);
        env.edit(|s| s.change(change));
        assert!(matches!(
            service.authenticate(&certificate.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(matches!(
            service.session_status(&certificate.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(matches!(
            service.record_activity(&certificate.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        let reads = env.read(|s| s.authority_reads);
        assert_eq!(
            service.authenticate(&password.access_token).unwrap(),
            password.principal
        );
        service.session_status(&password.access_token).unwrap();
        service.record_activity(&password.access_token).unwrap();
        env.read(|s| {
            assert_eq!(s.authority_reads, reads);
            assert_eq!(s.touches, 1);
        });
    }
}

#[test]
fn full_principal_changes_between_nested_reads_cannot_admit_a_certificate_session() {
    let env = Env::new();
    let service = env.service(true, SessionPolicy::default());
    let session = env.certificate_session(&service);
    env.edit(|s| s.after_user_find = Some(Change::Principal));
    assert!(matches!(
        service.authenticate(&session.access_token),
        Err(ApplicationError::InvalidSession)
    ));
    env.read(|s| assert_eq!(s.user.auth_generation, 4));
}

#[test]
fn certificate_activity_compares_origin_and_rechecks_authority_after_atomic_touch() {
    for change in [
        Change::Trust,
        Change::Withdraw,
        Change::Principal,
        Change::Advance(2000),
    ] {
        let env = Env::new();
        let policy = SessionPolicy::new(3600, Some(120)).unwrap();
        let service = env.service(true, policy);
        let session = env.certificate_session(&service);
        env.edit(|s| {
            s.now = 1050;
            s.after_touch = Some(change);
        });
        assert!(
            matches!(
                service.record_activity(&session.access_token),
                Err(ApplicationError::InvalidSession)
            ),
            "{change:?}"
        );
        env.read(|s| assert_eq!(s.touches, 1));
    }
}

#[test]
fn idle_activity_never_extends_the_original_credential_or_policy_ceiling() {
    for (absolute, expected) in [(3600, 2_000_000), (600, 1_600_000)] {
        let env = Env::new();
        let policy = SessionPolicy::new(absolute, Some(500)).unwrap();
        let service = env.service(true, policy);
        let session = env.certificate_session(&service);
        env.edit(|s| s.now = 1400);
        let first = service.record_activity(&session.access_token).unwrap();
        assert_eq!(first.absolute_expires_at_unix_ms, expected);
        assert_eq!(first.idle_expires_at_unix_ms, Some(expected.min(1_900_000)));
        if expected > 1_900_000 {
            env.edit(|s| s.now = 1800);
            let middle = service.record_activity(&session.access_token).unwrap();
            assert_eq!(middle.idle_expires_at_unix_ms, Some(expected));
        }
        env.edit(|s| s.now = expected / 1000 - 1);
        let last = service.record_activity(&session.access_token).unwrap();
        assert_eq!(last.absolute_expires_at_unix_ms, expected);
        assert_eq!(last.idle_expires_at_unix_ms, Some(expected));
        env.edit(|s| s.now = expected / 1000);
        assert!(matches!(
            service.record_activity(&session.access_token),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn current_authority_failures_never_fall_back_to_cached_certificate_admission() {
    let env = Env::new();
    let service = env.service(true, SessionPolicy::default());
    let session = env.certificate_session(&service);
    env.edit(|s| s.fail_authority = true);
    assert!(service.authenticate(&session.access_token).is_err());
    assert!(service.record_activity(&session.access_token).is_err());
    env.read(|s| assert_eq!(s.touches, 0));
}
