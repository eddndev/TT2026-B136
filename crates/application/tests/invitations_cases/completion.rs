use super::support::*;

#[test]
fn only_confirmed_totp_creates_one_atomic_account_consumption_and_receipt_command() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    let enrollment = prepare(&service);
    let receipt = service
        .finish(&RAW_TOKEN, operation(30), PASSWORD, CODE)
        .unwrap();
    let state = state.lock().unwrap();
    assert_eq!(state.finish_commands.len(), 1);
    let command = &state.finish_commands[0];
    assert_eq!(command.id, id());
    assert_eq!(command.revision, 1);
    assert_eq!(command.pending_operation_id, enrollment.operation_id);
    assert_eq!(command.operation_id, operation(30));
    assert_eq!(command.token_digest, token_digest(&state));
    assert_eq!(command.at, now());
    assert_eq!(command.user.email, EMAIL);
    assert_eq!(command.user.role, Role::Litigator);
    assert!(command.user.active);
    assert_eq!(command.user.password_hash, password_hash(PASSWORD));
    assert_eq!(command.user.protected_totp_secret, vec![0xb6, 7]);
    assert_eq!(command.user.recovery_codes.remaining(), RECOVERY_CODE_COUNT);
    assert_eq!(command.user.revision, 0);
    assert_eq!(command.user.auth_generation, 0);
    assert_eq!(receipt.user_id, command.user.id);
    assert_eq!(receipt.email, EMAIL);
    assert_eq!(state.enrollments, 1);
    assert_eq!(state.recoveries, 1);
    let checked = state
        .calls
        .iter()
        .position(|value| *value == "verify_totp")
        .unwrap();
    let committed = state
        .calls
        .iter()
        .position(|value| *value == "finish")
        .unwrap();
    assert!(checked < committed);
}

#[test]
fn wrong_totp_leaves_pending_enrollment_and_user_absent() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    assert!(matches!(
        service.finish(&RAW_TOKEN, operation(30), PASSWORD, "000000"),
        Err(ApplicationError::MfaRejected)
    ));
    let state = state.lock().unwrap();
    assert!(state.user.is_none());
    assert!(state.finish_commands.is_empty());
    assert!(state.candidate.as_ref().unwrap().pending.is_some());
}

#[test]
fn wrong_password_cannot_confirm_the_second_factor() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    assert!(matches!(
        service.finish(
            &RAW_TOKEN,
            operation(30),
            "a different valid password",
            CODE
        ),
        Err(ApplicationError::InvalidCredentials)
    ));
    let state = state.lock().unwrap();
    assert!(state.finish_commands.is_empty());
    assert!(!state.calls.contains(&"verify_totp"));
}

#[test]
fn expiry_or_resend_rejection_in_commit_cannot_be_reported_as_success_or_retried() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    state.lock().unwrap().finish_error = true;
    assert!(matches!(
        service.finish(&RAW_TOKEN, operation(30), PASSWORD, CODE),
        Err(ApplicationError::InvalidCredentials)
    ));
    let state = state.lock().unwrap();
    assert!(state.user.is_none());
    assert!(state.completion.is_none());
    assert_eq!(state.finish_commands.len(), 1);
}

#[test]
fn lost_finish_response_is_recovered_with_link_password_and_public_receipt() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    state.lock().unwrap().lose_finish_response = true;
    assert!(matches!(
        service.finish(&RAW_TOKEN, operation(30), PASSWORD, CODE),
        Err(ApplicationError::Port(_))
    ));
    let receipt = service
        .receipt(&RAW_TOKEN, operation(30), PASSWORD)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.id, id());
    assert_eq!(receipt.operation_id, operation(30));
    assert_eq!(receipt.email, EMAIL);
    assert_eq!(receipt.role, Role::Litigator);
    let state = state.lock().unwrap();
    assert_eq!(state.finish_commands.len(), 1);
    assert_eq!(state.enrollments, 1);
    assert_eq!(state.recoveries, 1);
    assert!(state.candidate.is_none());
}

#[test]
fn completion_replay_returns_original_receipt_but_a_changed_payload_is_rejected() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    let first = service
        .finish(&RAW_TOKEN, operation(30), PASSWORD, CODE)
        .unwrap();
    let repeated = service
        .finish(&RAW_TOKEN, operation(30), PASSWORD, CODE)
        .unwrap();
    assert_eq!(repeated, first);
    assert!(service
        .finish(&RAW_TOKEN, operation(30), PASSWORD, "654321")
        .is_err());
    assert!(service
        .finish(
            &RAW_TOKEN,
            operation(30),
            "a different valid password",
            CODE
        )
        .is_err());
    assert!(service
        .finish(&RAW_TOKEN, operation(31), PASSWORD, CODE)
        .is_err());
    assert_eq!(state.lock().unwrap().finish_commands.len(), 1);
}

#[test]
fn receipt_requires_the_resulting_accounts_current_password_and_never_returns_secrets() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let service = harness.service();
    prepare(&service);
    service
        .finish(&RAW_TOKEN, operation(30), PASSWORD, CODE)
        .unwrap();
    assert!(matches!(
        service.receipt(&RAW_TOKEN, operation(30), "a different valid password"),
        Err(ApplicationError::InvalidCredentials)
    ));
    state.lock().unwrap().user.as_mut().unwrap().password_hash =
        password_hash("replacement account password");
    assert!(matches!(
        service.receipt(&RAW_TOKEN, operation(30), PASSWORD),
        Err(ApplicationError::InvalidCredentials)
    ));
    let receipt = service
        .receipt(&RAW_TOKEN, operation(30), "replacement account password")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.email, EMAIL);
    let state = state.lock().unwrap();
    assert_eq!(state.finish_commands.len(), 1);
    assert_eq!(state.enrollments, 1);
    assert_eq!(state.recoveries, 1);
}
