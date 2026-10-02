use application::identity::password_reset::{
    ResetDeliveryOutcome, ResetPolicy, ResetRequestAccepted,
};
use application::ApplicationError;

use super::password_reset_support::*;

#[test]
fn policy_requires_explicit_positive_lifetime_and_capacity() {
    assert!(matches!(
        ResetPolicy::new(0, 2),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        ResetPolicy::new(60, 0),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert!(ResetPolicy::new(60, 2).is_ok());
}

#[test]
fn policy_rejects_durations_outside_integer_or_timestamp_capacity() {
    for seconds in [u64::MAX, i64::MAX as u64] {
        assert!(matches!(
            ResetPolicy::new(seconds, 2),
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
}

#[test]
fn issuance_normalizes_email_persists_only_scoped_digest_and_then_delivers_the_secret() {
    let (service, ports) = fixture();
    assert_eq!(
        service.request("  MEMBER@EXAMPLE.TEST  ").unwrap(),
        ResetRequestAccepted
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(
        state.calls,
        ["request_limit", "token", "digest", "issue", "delivery"]
    );
    assert_eq!(state.requests, [EMAIL]);
    assert_eq!(state.issues.len(), 1);
    assert_eq!(state.issues[0].email, EMAIL);
    assert_eq!(state.issues[0].digest, digest());
    assert_eq!(state.issues[0].policy, policy());
    let expected = [b"qadra:password-reset:v1\0".as_slice(), TOKEN.as_slice()].concat();
    assert_eq!(state.digest_inputs, [expected]);
    assert_eq!(state.delivered.len(), 1);
    assert_eq!(state.delivered[0].email, EMAIL);
    assert_eq!(state.delivered[0].token, TOKEN);
    assert_eq!(state.delivered[0].expires_at, deadline());
    assert!(state.hashed_passwords.is_empty());
    assert!(state.completions.is_empty());
    assert!(state.cancellations.is_empty());
}

#[test]
fn ignored_repository_issuance_has_the_same_public_result_without_delivery() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().ignore_issue = true;
    assert_eq!(service.request(EMAIL).unwrap(), ResetRequestAccepted);
    let state = ports.0.lock().unwrap();
    assert_eq!(state.issues.len(), 1);
    assert!(state.delivered.is_empty());
    assert!(state.cancellations.is_empty());
}

#[test]
fn request_limit_uses_normalized_email_and_stops_before_entropy_or_persistence() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().request_allowed = false;
    assert_eq!(
        service.request(" MEMBER@example.test ").unwrap(),
        ResetRequestAccepted
    );
    let state = ports.0.lock().unwrap();
    assert_eq!(state.calls, ["request_limit"]);
    assert_eq!(state.requests, [EMAIL]);
    assert!(state.delivered.is_empty());
}

#[test]
fn malformed_email_is_rejected_before_rate_limit_or_issuance() {
    for email in [
        "bad",
        "bad@@example.test",
        "a\n@example.test",
        "a b@example.test",
    ] {
        let (service, ports) = fixture();
        assert!(matches!(
            service.request(email),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert!(ports.0.lock().unwrap().calls.is_empty());
    }
}

#[test]
fn definitely_rejected_delivery_cancels_only_the_just_issued_id_and_digest() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().delivery_result = ResetDeliveryOutcome::DefinitelyRejected;
    assert_eq!(service.request(EMAIL).unwrap(), ResetRequestAccepted);
    let state = ports.0.lock().unwrap();
    assert_eq!(state.cancellations, [(issue_id(101), digest())]);
    assert_eq!(state.issues.len(), 1);
    assert_eq!(state.delivered.len(), 1);
    assert!(state.completions.is_empty());
}

#[test]
fn late_delivery_rejection_cannot_cancel_a_new_issuance_reusing_the_digest() {
    let (service, ports) = fixture();
    {
        let mut state = ports.0.lock().unwrap();
        state.delivery_result = ResetDeliveryOutcome::DefinitelyRejected;
        state.replace_issue_during_delivery = true;
    }
    assert_eq!(service.request(EMAIL).unwrap(), ResetRequestAccepted);
    let state = ports.0.lock().unwrap();
    assert_eq!(state.issuance_id, issue_id(202));
    assert_eq!(state.cancellations, [(issue_id(101), digest())]);
    assert_eq!(state.issues.len(), 1);
    assert_eq!(state.delivered.len(), 1);
    assert!(!state.calls.contains(&"inspect"));
}

#[test]
fn uncertain_delivery_keeps_the_capability_without_retry_or_another_issue() {
    let (service, ports) = fixture();
    ports.0.lock().unwrap().delivery_result = ResetDeliveryOutcome::Uncertain;
    assert_eq!(service.request(EMAIL).unwrap(), ResetRequestAccepted);
    let state = ports.0.lock().unwrap();
    assert_eq!(state.issues.len(), 1);
    assert_eq!(state.delivered.len(), 1);
    assert!(state.cancellations.is_empty());
}

#[test]
fn failed_ports_never_deliver_before_a_confirmed_issue_or_retry_automatically() {
    for failure in ["request_limit", "token", "issue", "delivery", "cancel"] {
        let (service, ports) = fixture();
        {
            let mut state = ports.0.lock().unwrap();
            state.fail_at = Some(failure);
            state.delivery_result = ResetDeliveryOutcome::DefinitelyRejected;
        }
        assert_port_failure(service.request(EMAIL).unwrap_err());
        let state = ports.0.lock().unwrap();
        assert_eq!(
            state.calls.iter().filter(|call| **call == failure).count(),
            1
        );
        if ["request_limit", "token", "issue"].contains(&failure) {
            assert!(!state.calls.contains(&"delivery"));
        }
        if failure == "delivery" {
            assert!(!state.calls.contains(&"cancel"));
        }
        assert!(state.completions.is_empty());
    }
}
