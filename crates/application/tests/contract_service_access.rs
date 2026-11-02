//! Contract reads by relation and the refusals that precede any signature
//! work, over in-memory doubles: docs/adr/0086-bilateral-contract-signing.md.
#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
mod contract_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod document_signing_support;

use application::contracts::*;
use contract_support::{client, contract_error, firm, store::*, FakeStore, Harness};
use std::sync::Arc;
use uuid::Uuid;
use EventKind::*;

fn harness(principal: application::identity::Principal, kinds: &[EventKind]) -> Harness {
    let mut harness = Harness::new(principal);
    harness.store = Arc::new(FakeStore::with(snapshot(kinds)));
    harness
}

fn without_scope(harness: &Harness) {
    harness.store.state().snapshot.as_mut().unwrap().case_scope = false;
}

#[test]
fn another_request_of_a_shared_contract_reads_as_not_found_to_a_party() {
    // The store gives a party without case scope only the requests that
    // designate it, so a later request for another Client is absent.
    let party = harness(client(), &[Rejected]);
    without_scope(&party);
    let other = Uuid::from_u128(REQUEST + 1);
    let service = party.service();
    assert_eq!(
        contract_error(service.request("session", other)),
        ContractWorkflowError::NotFound
    );
    assert_eq!(
        contract_error(service.prepare("session", other, Uuid::from_u128(1))),
        ContractWorkflowError::NotFound
    );
    assert_eq!(
        contract_error(service.export("session", other, ContractStage::Partial)),
        ContractWorkflowError::NotFound
    );
    let staff = harness(firm(), &[Requested]);
    assert_eq!(
        contract_error(staff.service().request("session", other)),
        ContractWorkflowError::Inconsistent,
        "a snapshot with case scope holds every request"
    );
}
