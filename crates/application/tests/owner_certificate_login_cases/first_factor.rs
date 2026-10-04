use domain::{crypto::DocumentHasher, identity::Role};

use crate::support::*;

#[test]
fn password_records_keep_their_exact_json_and_reject_an_added_origin() {
    let principal = Principal::from(&user());
    let challenge = LoginChallengeIdentity {
        user_id: principal.id,
        auth_generation: 4,
    };
    let session = SessionIdentity {
        principal: principal.clone(),
        auth_generation: 4,
    };
    assert_eq!(
        serde_json::to_value(&challenge).unwrap(),
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000001", "auth_generation": 4
        })
    );
    assert_eq!(
        serde_json::to_value(&session).unwrap(),
        serde_json::json!({
            "principal": {"id": "00000000-0000-0000-0000-000000000001",
                "email": "owner@example.test", "role": "owner"}, "auth_generation": 4
        })
    );
    let mut unknown = serde_json::to_value(&session).unwrap();
    unknown["authentication"] = serde_json::json!("certificate");
    assert!(serde_json::from_value::<SessionIdentity>(unknown).is_err());
}

#[test]
fn a_single_use_proof_requires_mfa_with_its_own_window_and_a_credential_session_ceiling() {
    let env = Env::new();
    let service = env.service(true, SessionPolicy::default());
    let challenge = env.begin(&service);
    assert_eq!(challenge.expires_in_seconds, 300);
    assert_eq!(challenge.statement.issued_at_unix_seconds(), 1000);
    assert_eq!(challenge.statement.expires_at_unix_seconds(), 1300);
    env.read(|s| {
        assert_eq!(
            s.proofs[&challenge.challenge_token].statement,
            challenge.statement
        )
    });
    env.edit(|s| s.now = 1299);
    let mfa = service
        .prove_certificate_login(&challenge.challenge_token, &[5; 384])
        .unwrap();
    assert_eq!(mfa.expires_in_seconds, 300);
    env.read(|s| {
        assert!(s.proofs.is_empty());
        assert!(s.sessions.is_empty());
        let MfaChallenge::Certificate(value) = &s.mfa[&mfa.challenge_token].0 else {
            panic!("certificate proof must not become password MFA");
        };
        assert_eq!(value.expires_at_unix_seconds, 1599);
        assert_eq!(value.provenance.valid_until_unix_seconds, 2000);
        assert_eq!(
            value.provenance.crl_digest,
            context().trust.inspection.crl_digest
        );
    });
    assert!(matches!(
        service.prove_certificate_login(&challenge.challenge_token, &[5; 384]),
        Err(ApplicationError::InvalidCredentials)
    ));
    env.edit(|s| s.now = 1500);
    let session = service
        .complete_totp(&mfa.challenge_token, "123456")
        .unwrap();
    assert_eq!(session.session.absolute_expires_at_unix_ms, 2_000_000);
    assert_eq!(session.expires_in_seconds, 500);
    env.read(|s| {
        let SessionAuthentication::Certificate(origin) =
            &s.sessions[&session.access_token].1.authentication
        else {
            panic!("certificate origin must survive MFA");
        };
        assert_eq!(origin.principal, session.principal);
        assert_eq!(origin.auth_generation, 4);
        assert_eq!(origin.binding_id, binding());
        assert_eq!(origin.trust_revision, 7);
        assert_eq!(origin.leaf_fingerprint, context().certificate.fingerprint);
        assert_eq!(
            origin.root_fingerprint,
            context().trust.inspection.root_fingerprint
        );
        assert_eq!(origin.crl_digest, context().trust.inspection.crl_digest);
        assert_eq!(origin.deployment_id, context().trust.deployment_id);
        assert_eq!(origin.valid_from_unix_seconds, 100);
        assert_eq!(s.verifier_calls, 1);
    });
    env.edit(|s| s.now = 1700);
    assert_eq!(
        service.authenticate(&session.access_token).unwrap(),
        session.principal
    );
    env.edit(|s| s.now = 2000);
    assert!(matches!(
        service.authenticate(&session.access_token),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn failed_and_wrong_length_proofs_are_consumed_without_a_second_crypto_attempt() {
    for length in [0, 383, 384, 385] {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let challenge = env.begin(&service);
        env.edit(|s| s.fail_verify = true);
        assert!(
            matches!(
                service.prove_certificate_login(&challenge.challenge_token, &vec![5; length]),
                Err(ApplicationError::InvalidCredentials)
            ),
            "length {length}"
        );
        env.edit(|s| s.fail_verify = false);
        assert!(matches!(
            service.prove_certificate_login(&challenge.challenge_token, &[5; 384]),
            Err(ApplicationError::InvalidCredentials)
        ));
        env.read(|s| {
            assert_eq!(s.verifier_calls, usize::from(length == 384));
            assert!(s.proofs.is_empty());
            assert!(s.mfa.is_empty());
        });
    }
}

#[test]
fn admission_budgets_stop_creation_and_crypto_before_expensive_work() {
    let env = Env::new();
    let service = env.service(true, SessionPolicy::default());
    env.edit(|s| s.fail_start = true);
    assert!(matches!(
        service.start_certificate_login(user().id, binding()),
        Err(ApplicationError::AccountLocked)
    ));
    env.read(|s| {
        assert_eq!(s.start_admissions, 1);
        assert!(s.proofs.is_empty());
    });
    env.edit(|s| s.fail_start = false);
    let challenge = env.begin(&service);
    env.edit(|s| s.fail_proof = true);
    assert!(matches!(
        service.prove_certificate_login(&challenge.challenge_token, &[5; 384]),
        Err(ApplicationError::AccountLocked)
    ));
    env.read(|s| {
        assert_eq!(s.proof_admissions, 1);
        assert_eq!(s.verifier_calls, 0);
    });
}

#[test]
fn start_rejects_unavailable_or_invalid_current_authority_without_a_capture() {
    for invalid in 0..5 {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        env.edit(|s| match invalid {
            0 => s.context = None,
            1 => s.context.as_mut().unwrap().account.principal.role = Role::Litigator,
            2 => s.context.as_mut().unwrap().account.active = false,
            3 => s.context.as_mut().unwrap().certificate.der = vec![1; 16_385],
            _ => s.now = 2000,
        });
        assert!(
            matches!(
                service.start_certificate_login(user().id, binding()),
                Err(ApplicationError::InvalidCredentials)
            ),
            "authority case {invalid}"
        );
        env.read(|s| {
            assert!(s.proofs.is_empty());
            assert_eq!(s.verifier_calls, 0);
        });
    }
}

#[test]
fn cryptographic_port_results_must_match_every_captured_material_and_time() {
    for fault in [
        CheckFault::Digest,
        CheckFault::Signature,
        CheckFault::Certificate,
        CheckFault::Trust,
        CheckFault::Time,
        CheckFault::Window,
    ] {
        let env = Env::new();
        let service = env.service(true, SessionPolicy::default());
        let challenge = env.begin(&service);
        env.edit(|s| s.check_fault = Some(fault));
        assert!(
            matches!(
                service.prove_certificate_login(&challenge.challenge_token, &[5; 384]),
                Err(ApplicationError::InvalidCredentials)
            ),
            "{fault:?}"
        );
        env.read(|s| {
            assert_eq!(s.verifier_calls, 1);
            assert!(s.mfa.is_empty());
        });
    }
}

#[test]
fn changed_capture_is_rejected_before_rsa_and_after_rsa_or_mfa_storage() {
    for phase in 0..3 {
        for change in [
            Change::Trust,
            Change::Withdraw,
            Change::Principal,
            Change::Generation,
            Change::Advance(2000),
        ] {
            let env = Env::new();
            let service = env.service(true, SessionPolicy::default());
            let challenge = env.begin(&service);
            env.edit(|s| match phase {
                0 => s.change(change),
                1 => s.after_verify = Some(change),
                _ => s.after_mfa_create = Some(change),
            });
            assert!(
                matches!(
                    service.prove_certificate_login(&challenge.challenge_token, &[5; 384]),
                    Err(ApplicationError::InvalidCredentials)
                ),
                "phase {phase}, {change:?}"
            );
            env.read(|s| {
                assert_eq!(s.verifier_calls, usize::from(phase != 0));
                assert!(s.mfa.is_empty());
                assert!(s.sessions.is_empty());
            });
        }
    }
}

#[test]
fn a_same_revision_trust_digest_change_cannot_reuse_a_stored_proof() {
    let env = Env::new();
    let service = env.service(true, SessionPolicy::default());
    let challenge = env.begin(&service);
    env.edit(|s| {
        let trust = &mut s.context.as_mut().unwrap().trust;
        trust.inspection.crl_der = b"different-crl-at-same-revision".to_vec();
        trust.inspection.crl_digest = Hasher.hash_bytes(&trust.inspection.crl_der);
    });
    assert!(matches!(
        service.prove_certificate_login(&challenge.challenge_token, &[5; 384]),
        Err(ApplicationError::InvalidCredentials)
    ));
    env.read(|s| assert_eq!(s.verifier_calls, 0));
}
