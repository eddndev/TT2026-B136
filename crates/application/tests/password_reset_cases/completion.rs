use application::identity::password_reset::ResetCompletion;
use application::ApplicationError;

use super::password_reset_support::*;

#[test]
fn valid_completion_passes_one_atomic_command_with_candidate_generation_and_hash() {
    let (service, ports) = fixture();
    assert_eq!(
        service.complete(&TOKEN, PASSWORD).unwrap(),
        ResetCompletion::Changed
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(
        state.calls,
        ["digest", "completion_limit", "inspect", "hash", "complete"]
    );
    assert_eq!(state.completions.len(), 1);
    let command = &state.completions[0];
    assert_eq!(command.digest, digest());
    assert_eq!(command.candidate.id, issue_id(101));
    assert_eq!(command.candidate.user_id, candidate().user_id);
    assert_eq!(command.candidate.auth_generation, 7);
    assert_eq!(command.password_hash, HASH);
    assert_eq!(state.hashed_passwords, [PASSWORD]);
    assert!(state.delivered.is_empty());
    assert!(state.issues.is_empty());
    assert!(state.cancellations.is_empty());
}

#[test]
fn consumption_preserves_the_exact_issuance_id_observed_before_password_hashing() {
    let (service, ports) = fixture();
    {
        let mut state = ports.0.lock().unwrap();
        state.candidate.as_mut().unwrap().id = issue_id(303);
        state.reject_after_hash = true;
    }
    assert_eq!(
        service.complete(&TOKEN, PASSWORD).unwrap(),
        ResetCompletion::Rejected
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(state.completions.len(), 1);
    assert_eq!(state.completions[0].candidate.id, issue_id(303));
    assert_eq!(state.completions[0].digest, digest());
    assert_eq!(
        state
            .calls
            .iter()
            .filter(|call| **call == "inspect")
            .count(),
        1
    );
    assert!(state.cancellations.is_empty());
    assert!(state.issues.is_empty());
}

#[test]
fn completion_hashes_the_same_purpose_and_32_bytes_used_by_issuance() {
    let (service, ports) = fixture();
    service.request(EMAIL).unwrap();
    service.complete(&TOKEN, PASSWORD).unwrap();
    let state = ports.0.lock().unwrap();
    assert_eq!(state.digest_inputs.len(), 2);
    assert_eq!(state.digest_inputs[0], state.digest_inputs[1]);
    assert_eq!(state.completion_limits, [digest()]);
}

#[test]
fn malformed_capability_lengths_are_rejected_without_hashing_or_store_access() {
    for length in [0, 1, 31, 33, 1024] {
        let (service, ports) = fixture();
        assert_eq!(
            service.complete(&vec![0x5a; length], PASSWORD).unwrap(),
            ResetCompletion::Rejected
        );
        assert!(ports.0.lock().unwrap().calls.is_empty());
    }
}

#[test]
fn existing_password_byte_limits_reject_without_consuming_the_capability() {
    for password in ["a".repeat(11), "a".repeat(1025)] {
        let (service, ports) = fixture();
        assert!(matches!(
            service.complete(&TOKEN, &password),
            Err(ApplicationError::InvalidInput(_))
        ));
        let state = ports.0.lock().unwrap();
        assert!(state.hashed_passwords.is_empty());
        assert!(state.completions.is_empty());
        assert!(state.cancellations.is_empty());
    }
}

#[test]
fn password_boundaries_count_bytes_and_preserve_the_exact_chosen_value() {
    for password in ["a".repeat(12), "a".repeat(1024), "\u{e9}".repeat(6)] {
        let (service, ports) = fixture();
        assert_eq!(
            service.complete(&TOKEN, &password).unwrap(),
            ResetCompletion::Changed
        );
        let state = ports.0.lock().unwrap();
        assert_eq!(state.hashed_passwords, [password]);
        assert_eq!(state.completions.len(), 1);
    }
}

#[test]
fn completion_limit_stops_before_candidate_lookup_and_password_hashing() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().completion_allowed = false;
    assert_eq!(
        service.complete(&TOKEN, PASSWORD).unwrap(),
        ResetCompletion::Rejected
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(state.calls, ["digest", "completion_limit"]);
    assert!(state.completions.is_empty());
}

#[test]
fn missing_or_unavailable_candidate_is_generic_and_does_not_hash_the_password() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().candidate = None;
    assert_eq!(
        service.complete(&TOKEN, PASSWORD).unwrap(),
        ResetCompletion::Rejected
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(state.calls, ["digest", "completion_limit", "inspect"]);
    assert!(state.completions.is_empty());
}

#[test]
fn repository_rejection_after_hash_is_authoritative_and_never_reissues_a_token() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().reject_after_hash = true;
    assert_eq!(
        service.complete(&TOKEN, PASSWORD).unwrap(),
        ResetCompletion::Rejected
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(state.completions.len(), 1);
    assert_eq!(state.completions[0].candidate.auth_generation, 7);
    assert_eq!(state.hashed_passwords.len(), 1);
    assert!(state.issues.is_empty());
    assert!(state.delivered.is_empty());
    assert!(state.cancellations.is_empty());
}

#[test]
fn password_hash_failure_leaves_consumption_and_delivery_untouched() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().hash_fails = true;
    assert!(matches!(
        service.complete(&TOKEN, PASSWORD),
        Err(ApplicationError::Domain(_))
    ));
    let state = ports.0.lock().unwrap();
    assert!(state.completions.is_empty());
    assert!(state.issues.is_empty());
    assert!(state.cancellations.is_empty());
}

#[test]
fn failure_of_the_atomic_port_is_not_reported_as_a_success_or_retried() {
    for failure in ["completion_limit", "inspect", "complete"] {
        let (service, ports) = fixture();
        ports.0.lock().unwrap().fail_at = Some(failure);
        assert_port_failure(service.complete(&TOKEN, PASSWORD).unwrap_err());
        let state = ports.0.lock().unwrap();
        assert_eq!(
            state.calls.iter().filter(|call| **call == failure).count(),
            1
        );
        assert!(state.issues.is_empty());
        assert!(state.delivered.is_empty());
        assert!(state.cancellations.is_empty());
    }
}
