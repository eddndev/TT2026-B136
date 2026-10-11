use super::support::*;

#[test]
fn capability_must_have_exactly_32_bytes_before_any_port_call() {
    for length in [0, 1, 31, 33, 1024] {
        let harness = Harness::new();
        let state = harness.state.clone();
        let service = harness.service();
        let token = vec![7; length];
        assert!(matches!(
            service.prepare(&token, operation(20), PASSWORD),
            Err(ApplicationError::InvalidCredentials)
        ));
        assert!(matches!(
            service.finish(&token, operation(30), PASSWORD, CODE),
            Err(ApplicationError::InvalidCredentials)
        ));
        assert!(matches!(
            service.receipt(&token, operation(30), PASSWORD),
            Err(ApplicationError::InvalidCredentials)
        ));
        assert!(state.lock().unwrap().calls.is_empty());
    }
}

#[test]
fn chosen_password_keeps_existing_byte_limits_without_creating_pending_material() {
    for password in ["a".repeat(11), "a".repeat(1025)] {
        let harness = Harness::new();
        let state = harness.state.clone();
        let service = harness.service();
        assert!(matches!(
            service.prepare(&RAW_TOKEN, operation(20), &password),
            Err(ApplicationError::InvalidInput(_))
        ));
        let state = state.lock().unwrap();
        assert!(state.hashes.is_empty());
        assert_eq!(state.pending_writes, 0);
    }
}

#[test]
fn missing_expired_and_exact_expiry_capabilities_do_not_enroll() {
    for expiry_value in [None, Some(now() - time::Duration::seconds(1)), Some(now())] {
        let harness = Harness::new();
        let state = harness.state.clone();
        {
            let mut state = state.lock().unwrap();
            match expiry_value {
                Some(value) => state.candidate.as_mut().unwrap().expires_at = value,
                None => state.candidate = None,
            }
        }
        let service = harness.service();
        assert!(matches!(
            service.prepare(&RAW_TOKEN, operation(20), PASSWORD),
            Err(ApplicationError::InvalidCredentials)
        ));
        let state = state.lock().unwrap();
        assert_eq!(state.enrollments, 0);
        assert!(state.hashes.is_empty());
        assert_eq!(state.pending_writes, 0);
    }
}

#[test]
fn invitation_limiter_stops_before_candidate_lookup_and_password_hashing() {
    let harness = Harness::new();
    let state = harness.state.clone();
    state.lock().unwrap().limited = true;
    let service = harness.service();
    assert!(matches!(
        service.prepare(&RAW_TOKEN, operation(20), PASSWORD),
        Err(ApplicationError::InvalidCredentials)
    ));
    let state = state.lock().unwrap();
    assert_eq!(state.calls, ["digest", "limit"]);
    assert!(state.hashes.is_empty());
    assert_eq!(state.enrollments, 0);
}

#[test]
fn preparation_saves_encrypted_pending_material_without_creating_a_usable_user() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    let result = prepare(&service);
    assert_eq!(result.id, id());
    assert_eq!(result.revision, 1);
    assert_eq!(result.operation_id, operation(20));
    assert_eq!(result.email, EMAIL);
    assert_eq!(result.role, Role::Litigator);
    assert_eq!(result.expires_at, expiry());
    assert_eq!(result.recovery_codes.len(), RECOVERY_CODE_COUNT);
    let state = state.lock().unwrap();
    assert!(state.user.is_none());
    assert!(state.finish_commands.is_empty());
    assert_eq!(state.pending_writes, 1);
    assert_eq!(state.enrollments, 1);
    assert_eq!(state.recoveries, 1);
    assert_eq!(
        state
            .hashes
            .iter()
            .filter(|value| *value == PASSWORD)
            .count(),
        1
    );
    let pending = state.candidate.as_ref().unwrap().pending.as_ref().unwrap();
    let sealed = state
        .sealed
        .iter()
        .find(|value| value.opaque == pending.protected_material)
        .unwrap();
    assert_eq!(
        sealed.context,
        InvitationProtectionContext {
            purpose: InvitationPurpose::PendingMfa,
            id: id(),
            revision: 1,
        }
    );
    assert!(!sealed
        .plaintext
        .windows(PASSWORD.len())
        .any(|part| part == PASSWORD.as_bytes()));
    assert_ne!(pending.protected_material, sealed.plaintext);
}

#[test]
fn link_and_same_password_restore_exact_mfa_material_without_regeneration() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    let first = prepare(&service);
    let repeated = service
        .prepare(&RAW_TOKEN, operation(20), PASSWORD)
        .unwrap();
    let reopened = service
        .prepare(&RAW_TOKEN, operation(21), PASSWORD)
        .unwrap();
    for restored in [repeated, reopened] {
        assert_eq!(restored.id, first.id);
        assert_eq!(restored.operation_id, first.operation_id);
        assert_eq!(*restored.totp_secret_base32, *first.totp_secret_base32);
        assert_eq!(*restored.otpauth_uri, *first.otpauth_uri);
        assert_eq!(restored.recovery_codes, first.recovery_codes);
    }
    let state = state.lock().unwrap();
    assert_eq!(state.enrollments, 1);
    assert_eq!(state.recoveries, 1);
    assert_eq!(state.pending_writes, 1);
    assert!(state.user.is_none());
}

#[test]
fn another_password_cannot_read_or_replace_pending_mfa_material() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    assert!(matches!(
        service.prepare(&RAW_TOKEN, operation(20), "a different valid password"),
        Err(ApplicationError::InvalidCredentials)
    ));
    assert!(matches!(
        service.prepare(&RAW_TOKEN, operation(21), "a different valid password"),
        Err(ApplicationError::InvalidCredentials)
    ));
    let state = state.lock().unwrap();
    assert_eq!(state.pending_writes, 1);
    assert_eq!(state.enrollments, 1);
    assert_eq!(state.recoveries, 1);
    assert!(state.user.is_none());
}

#[test]
fn pending_ciphertext_failure_does_not_regenerate_or_overwrite_secrets() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    state.lock().unwrap().corrupt_ciphertext = true;
    assert!(matches!(
        service.prepare(&RAW_TOKEN, operation(20), PASSWORD),
        Err(ApplicationError::Port(_))
    ));
    let state = state.lock().unwrap();
    assert_eq!(state.pending_writes, 1);
    assert_eq!(state.enrollments, 1);
    assert!(state.finish_commands.is_empty());
}
