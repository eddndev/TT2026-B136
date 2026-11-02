//! Contract service rules over in-memory doubles:
//! docs/adr/0086-bilateral-contract-signing.md.
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
use application::document_signing::SigningWorkflowError;
use contract_support::{
    client, contract_error, firm, signing_error, store::*, FakeStore, Harness, CREATED, DAY,
};
use domain::identity::{Role, UserId};
use std::sync::Arc;
use uuid::Uuid;
use EventKind::*;

const OPERATION: u128 = 7_000;

fn harness(principal: application::identity::Principal, kinds: &[EventKind]) -> Harness {
    let mut harness = Harness::new(principal);
    harness.store = Arc::new(FakeStore::with(snapshot(kinds)));
    harness
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
) -> Result<ContractReceipt, application::ApplicationError> {
    harness
        .service()
        .sign("session", Uuid::from_u128(REQUEST), value)
}

#[test]
fn the_client_statement_names_the_exact_request_and_commits_once_with_t1() {
    let harness = harness(client(), &[Requested]);
    let value = submitted(&harness, 7);
    let statement =
        domain::document_signing::contract::ContractPartyStatement::from_canonical_bytes(
            &value.statement,
        )
        .unwrap();
    assert_eq!(statement.request_digest(), request_digest());
    assert_eq!(statement.position().number(), 1);
    let receipt = sign(&harness, &value).unwrap();
    assert_eq!(receipt.event.kind, ClientSigned);
    assert_eq!(receipt.event.revision, 3);
    assert_eq!(harness.calls(), (1, 1, 1));
    let state = harness.store.state();
    let recipients: Vec<UserId> = state.commands[0]
        .notices
        .iter()
        .map(|n| n.recipient)
        .collect();
    assert_eq!(recipients, vec![client().id, firm().id]);
    match &state.commands[0].write {
        ContractWrite::Sign {
            timestamp,
            completion,
            ..
        } => {
            assert!(timestamp.is_some() && completion.is_none());
        }
        other => panic!("unexpected write {other:?}"),
    }
}

#[test]
fn an_exact_replay_returns_the_original_receipt_before_expiry_and_trust() {
    let harness = harness(client(), &[Requested]);
    let value = submitted(&harness, 7);
    let first = sign(&harness, &value).unwrap();
    harness.set(|world| world.now = CREATED + DAY + 5);
    assert_eq!(sign(&harness, &value).unwrap(), first);
    assert_eq!(harness.calls(), (1, 1, 1));
    let mut changed = value.clone();
    changed.signature = vec![8; 384];
    assert_eq!(
        signing_error(sign(&harness, &changed)),
        SigningWorkflowError::Conflict
    );
    assert_eq!(harness.calls(), (1, 1, 1));
}

#[test]
fn an_unavailable_authority_writes_nothing() {
    let harness = harness(client(), &[Requested]);
    harness.set(|world| world.tsa_failure = true);
    let value = submitted(&harness, 7);
    assert_eq!(
        contract_error(sign(&harness, &value)),
        ContractWorkflowError::TimestampUnavailable
    );
    assert_eq!(harness.calls(), (1, 1, 0));
}

#[test]
fn a_token_whose_accuracy_reaches_the_expiry_is_refused() {
    let harness = harness(client(), &[Requested]);
    harness.set(|world| world.gen_time = CREATED + DAY);
    let value = submitted(&harness, 7);
    assert_eq!(
        contract_error(sign(&harness, &value)),
        ContractWorkflowError::Expired
    );
    assert_eq!(harness.calls(), (1, 1, 0));
}

#[test]
fn turn_version_validity_and_revision_are_checked_before_any_signature_work() {
    let firm_first = harness(firm(), &[Requested]);
    let prepared = firm_first.service().prepare(
        "session",
        Uuid::from_u128(REQUEST),
        Uuid::from_u128(OPERATION),
    );
    assert_eq!(contract_error(prepared), ContractWorkflowError::OutOfTurn);

    let superseded = harness(client(), &[Requested]);
    superseded
        .store
        .state()
        .snapshot
        .as_mut()
        .unwrap()
        .latest_version = 2;
    let prepared = superseded.service().prepare(
        "session",
        Uuid::from_u128(REQUEST),
        Uuid::from_u128(OPERATION),
    );
    assert_eq!(
        contract_error(prepared),
        ContractWorkflowError::VersionSuperseded
    );

    let expired = harness(client(), &[Requested]);
    expired.set(|world| world.now = CREATED + DAY);
    let prepared = expired.service().prepare(
        "session",
        Uuid::from_u128(REQUEST),
        Uuid::from_u128(OPERATION),
    );
    assert_eq!(contract_error(prepared), ContractWorkflowError::Expired);

    let closed = harness(client(), &[Rejected]);
    let prepared = closed.service().prepare(
        "session",
        Uuid::from_u128(REQUEST),
        Uuid::from_u128(OPERATION),
    );
    assert_eq!(
        contract_error(prepared),
        ContractWorkflowError::RequestClosed
    );

    let stale = harness(client(), &[Requested]);
    let mut value = submitted(&stale, 7);
    value.expected_revision = 1;
    assert_eq!(
        contract_error(sign(&stale, &value)),
        ContractWorkflowError::RevisionConflict
    );
    assert_eq!(stale.calls(), (0, 0, 0));
}

#[test]
fn an_account_without_any_relation_learns_nothing() {
    let mut stranger = client();
    stranger.id = UserId::from_uuid(Uuid::from_u128(99));
    let harness = harness(stranger, &[Requested]);
    harness.store.state().snapshot.as_mut().unwrap().case_scope = false;
    let service = harness.service();
    assert_eq!(
        contract_error(service.request("session", Uuid::from_u128(REQUEST))),
        ContractWorkflowError::NotFound
    );
    assert_eq!(
        contract_error(service.prepare("session", Uuid::from_u128(REQUEST), Uuid::from_u128(1))),
        ContractWorkflowError::NotFound
    );
}

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
fn a_revoked_client_certificate_blocks_the_counter_signature_before_any_token() {
    let harness = client_signed();
    harness.set(|world| world.client_revoked = true);
    let value = submitted(&harness, 9);
    let statement =
        domain::document_signing::contract::ContractPartyStatement::from_canonical_bytes(
            &value.statement,
        )
        .unwrap();
    assert_eq!(statement.position().number(), 2);
    assert_eq!(
        contract_error(sign(&harness, &value)),
        ContractWorkflowError::CounterpartyRevoked
    );
    assert_eq!(harness.calls(), (1, 0, 0));
}

#[test]
fn request_blockers_never_tell_why_the_client_is_not_ready() {
    let harness = harness(firm(), &[]);
    harness.store.state().candidates = Some(RequestCandidates {
        client: None,
        firm_signer: Some((
            firm().id,
            Role::Litigator,
            document_signing_support::receipt(),
        )),
        document: None,
        trust: Some(document_signing_support::trust()),
    });
    let command = NewRequest {
        request: Uuid::from_u128(REQUEST),
        operation: Uuid::from_u128(OPERATION),
        expected_revision: 1,
        version: 1,
        client_email: "nobody@example.test".into(),
        firm_signer: firm().id,
        ttl_days: 7,
        ceremony_reference: "HF-2026-001".into(),
        subject: None,
    };
    let result = harness.service().create_request(
        "session",
        snapshot(&[]).root.case,
        snapshot(&[]).root.id,
        &command,
    );
    assert_eq!(
        contract_error(result),
        ContractWorkflowError::PartyIneligible(ContractBlockers {
            client_not_ready: true,
            firm_signer_not_ready: false,
            document_not_ready: true,
        })
    );
    assert_eq!(harness.calls(), (0, 0, 0));
}

#[test]
fn the_expiry_closer_writes_expired_events_without_an_actor_and_skips_lost_races() {
    let harness = Harness::new(firm());
    harness.store.state().exhausted = vec![snapshot(&[Requested])];
    harness.set(|world| world.now = CREATED + DAY);
    let clock = Arc::new(contract_support::fixed_clock(CREATED + DAY));
    let closer = ContractExpiryService::new(
        harness.store.clone(),
        clock.clone(),
        NoticeSettings {
            from: "qadra@example.test".into(),
            public_url: None,
        },
    );
    assert_eq!(closer.run_once().unwrap(), 1);
    {
        let state = harness.store.state();
        let command = &state.commands[0];
        assert!(command.actor.is_none());
        assert_eq!(command.revision, 3);
        assert!(matches!(
            command.write,
            ContractWrite::Close { kind: Expired, .. }
        ));
        assert_eq!(command.notices.len(), 2);
    }
    harness.store.state().conflict_on_commit = true;
    assert_eq!(closer.run_once().unwrap(), 0);
    let early = ContractExpiryService::new(
        harness.store.clone(),
        Arc::new(contract_support::fixed_clock(CREATED + DAY - 1)),
        NoticeSettings {
            from: "qadra@example.test".into(),
            public_url: None,
        },
    );
    harness.store.state().conflict_on_commit = false;
    assert_eq!(early.run_once().unwrap(), 0);
}
