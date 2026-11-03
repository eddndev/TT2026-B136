use super::*;
use crate::contracts::test_support::{request, snapshot};
use EventKind::*;

const DAY: i64 = 86_400;

fn states(kinds: &[EventKind], now: i64) -> (ContractState, Option<RequestStatus>) {
    let value = snapshot(kinds, vec![request(100, 0)]);
    let status = value
        .requests
        .first()
        .filter(|_| kinds.len() > 1)
        .map(|r| request_status(&value, r, now));
    (contract_state(&value, now), status)
}

#[test]
fn a_new_contract_is_a_draft_and_a_request_puts_it_in_signing() {
    assert_eq!(
        contract_state(&snapshot(&[Created], vec![]), 0),
        ContractState::Draft
    );
    assert_eq!(
        states(&[Created, Requested], 10),
        (ContractState::Signing, Some(RequestStatus::AwaitingClient))
    );
    assert_eq!(
        states(&[Created, Requested, ClientSigned], 10),
        (ContractState::Signing, Some(RequestStatus::AwaitingFirm))
    );
}

#[test]
fn an_exhausted_request_is_not_open_before_its_expired_event_exists() {
    for kinds in [
        &[Created, Requested][..],
        &[Created, Requested, ClientSigned][..],
    ] {
        let value = snapshot(kinds, vec![request(100, 0)]);
        assert!(open_request(&value, DAY - 1).is_some());
        assert!(open_request(&value, DAY).is_none());
        assert_eq!(contract_state(&value, DAY), ContractState::Draft);
        assert_eq!(
            request_status(&value, &value.requests[0], DAY),
            RequestStatus::Expired
        );
        assert!(unclosed_request(&value).is_some());
    }
}

#[test]
fn closing_events_return_to_draft_and_terminal_events_stay() {
    for (kind, status) in [
        (Rejected, RequestStatus::Rejected),
        (Cancelled, RequestStatus::Cancelled),
        (Expired, RequestStatus::Expired),
    ] {
        assert_eq!(
            states(&[Created, Requested, kind], 10),
            (ContractState::Draft, Some(status))
        );
    }
    assert_eq!(
        states(&[Created, Requested, ClientSigned, Declined], 10),
        (ContractState::Draft, Some(RequestStatus::Declined))
    );
    assert_eq!(
        states(&[Created, Requested, ClientSigned, Completed], DAY * 9),
        (ContractState::Signed, Some(RequestStatus::Completed))
    );
    assert_eq!(
        contract_state(&snapshot(&[Created, Abandoned], vec![]), 0),
        ContractState::Abandoned
    );
}

#[test]
fn state_names_are_the_stored_spanish_values() {
    assert_eq!(
        [
            ContractState::Draft,
            ContractState::Signing,
            ContractState::Signed,
            ContractState::Abandoned
        ]
        .map(ContractState::as_str),
        ["borrador", "en_firma", "firmado", "abandonado"]
    );
}
