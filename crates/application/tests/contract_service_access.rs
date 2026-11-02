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
use application::ApplicationError;
use contract_support::{client, contract_error, firm, store::*, FakeStore, Harness};
use std::sync::Arc;
use uuid::Uuid;
use EventKind::*;

const OPERATION: u128 = 7_000;

fn harness(principal: application::identity::Principal, kinds: &[EventKind]) -> Harness {
    let mut harness = Harness::new(principal);
    harness.store = Arc::new(FakeStore::with(snapshot(kinds)));
    harness
}

fn without_scope(harness: &Harness) {
    harness.store.state().snapshot.as_mut().unwrap().case_scope = false;
}

fn submitted(harness: &Harness, signature: u8) -> SubmittedSignature {
    let prepared = harness
        .service()
        .prepare(
            "session",
            Uuid::from_u128(REQUEST),
            Uuid::from_u128(OPERATION),
        )
        .unwrap();
    SubmittedSignature {
        operation: Uuid::from_u128(OPERATION),
        expected_revision: prepared.revision,
        statement: prepared.statement.canonical_bytes().to_vec(),
        signature: vec![signature; 384],
    }
}

fn sign(
    harness: &Harness,
    value: &SubmittedSignature,
) -> Result<ContractReceipt, ApplicationError> {
    harness
        .service()
        .sign("session", Uuid::from_u128(REQUEST), value)
}

/// The firm signer facing a request the Client already signed.
fn client_signed() -> Harness {
    let client_side = harness(client(), &[Requested]);
    let value = submitted(&client_side, 7);
    let receipt = sign(&client_side, &value).unwrap();
    let mut snapshot = snapshot(&[Requested, ClientSigned]);
    snapshot.requests[0]
        .signatures
        .push(receipt.signature.unwrap());
    let harness = Harness::new(firm());
    Harness {
        store: Arc::new(FakeStore::with(snapshot)),
        ..harness
    }
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

#[test]
fn a_firm_signer_without_case_scope_reads_no_content_or_package() {
    let harness = harness(firm(), &[Requested]);
    without_scope(&harness);
    let service = harness.service();
    let request = Uuid::from_u128(REQUEST);
    assert!(service.request("session", request).is_ok());
    assert_eq!(
        contract_error(service.content("session", request).map(|_| ())),
        ContractWorkflowError::NotFound
    );
    for stage in [ContractStage::Partial, ContractStage::Final] {
        assert_eq!(
            contract_error(service.export("session", request, stage)),
            ContractWorkflowError::NotFound
        );
    }
}

#[test]
fn a_firm_signer_without_case_scope_cannot_sign_or_decline_before_any_work() {
    let harness = client_signed();
    let value = submitted(&harness, 9);
    without_scope(&harness);
    let refused = sign(&harness, &value);
    assert!(
        matches!(refused, Err(ApplicationError::PermissionDenied)),
        "{refused:?}"
    );
    assert_eq!(harness.calls(), (0, 0, 0));
    let prepared = harness.service().prepare(
        "session",
        Uuid::from_u128(REQUEST),
        Uuid::from_u128(OPERATION + 1),
    );
    assert!(matches!(prepared, Err(ApplicationError::PermissionDenied)));
    let declined = harness.service().close(
        "session",
        Uuid::from_u128(REQUEST),
        Uuid::from_u128(OPERATION + 2),
        3,
        CloseOutcome::Decline,
        None,
    );
    assert!(matches!(declined, Err(ApplicationError::PermissionDenied)));
    assert_eq!(harness.calls(), (0, 0, 0));
}
