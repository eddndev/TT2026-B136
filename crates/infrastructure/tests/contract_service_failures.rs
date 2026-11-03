//! Refusals and closings of the contract workflow over PostgreSQL: an
//! unavailable authority, a withdrawn Client binding, a superseded version,
//! an exhausted request and the expiry closer:
//! docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_service_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{contracts::*, documents::CaseDocumentStore};
use contract_service_support::{contract_error, Live, DAY};
use contract_storage_support::opens;
use domain::{clock::Clock as _, crypto::DocumentVersion};
use infrastructure::PostgresCaseDocumentStore;
use signing_binding_support::TOKEN;
use std::sync::atomic::Ordering;
use uuid::Uuid;

fn closer(live: &Live) -> ContractExpiryService {
    ContractExpiryService::new(
        live.store(),
        live.clock.clone(),
        NoticeSettings {
            from: "qadra@example.test".into(),
            public_url: None,
        },
    )
}

#[test]
fn an_unavailable_authority_leaves_no_state_and_the_same_bytes_succeed_later() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let client = live.client();
    let submitted = live.signed(&client, Uuid::new_v4());
    let before = (
        live.count("contract_events"),
        live.count("contract_party_signatures"),
    );
    live.authority.down.store(true, Ordering::SeqCst);
    assert_eq!(
        contract_error(live.sign(&client, &submitted)),
        ContractWorkflowError::TimestampUnavailable
    );
    let after = (
        live.count("contract_events"),
        live.count("contract_party_signatures"),
    );
    assert_eq!(before, after);
    assert_eq!(live.count("contract_notifications"), 1);
    live.authority.down.store(false, Ordering::SeqCst);
    let receipt = live.sign(&client, &submitted).unwrap();
    assert_eq!(receipt.event.kind, EventKind::ClientSigned);
    assert!(opens(&live.test));
}

#[test]
fn a_client_binding_withdrawn_after_its_signature_does_not_block_completion() {
    let Some(live) = Live::new() else { return };
    let live = live.requested();
    let (client, firm) = (live.client(), live.firm());
    live.sign(&client, &live.signed(&client, Uuid::new_v4()))
        .unwrap();
    live.test
        .arc_clock()
        .set(live.clock.now().unix_timestamp() + 1);
    let bindings = signing_binding_support::store(&live.test.individual.db, &live.test.arc_clock());
    signing_binding_support::service(bindings, &live.test.arc_clock(), client.clone())
        .withdraw(
            TOKEN,
            live.test.client_binding.statement.material().binding(),
        )
        .unwrap();
    let completed = live
        .sign(&firm, &live.signed(&firm, Uuid::new_v4()))
        .unwrap();
    assert_eq!(completed.event.kind, EventKind::Completed);
    assert!(opens(&live.test));
}

#[test]
fn a_later_version_supersedes_the_request() {
    let Some(live) = Live::new() else { return };
    let live = live.requested();
    let client = live.client();
    let submitted = live.signed(&client, Uuid::new_v4());
    let record = individual_document_backend_support::processor::processor()
        .prepare_version(
            live.test.individual.record.id,
            DocumentVersion::new(2).unwrap(),
            "reviewed.pdf",
            b"%PDF-1.7\n% second version\n%%EOF\n",
        )
        .unwrap();
    PostgresCaseDocumentStore::open(&live.test.individual.db.runtime_url)
        .unwrap()
        .append(
            live.firm().id,
            live.test.individual.db.case,
            DocumentVersion::new(1).unwrap(),
            record,
            live.clock.now(),
        )
        .unwrap();
    assert_eq!(
        contract_error(live.sign(&client, &submitted)),
        ContractWorkflowError::VersionSuperseded
    );
}

#[test]
fn an_exhausted_request_cannot_be_signed_and_the_closer_records_its_expiry() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let client = live.client();
    let submitted = live.signed(&client, Uuid::new_v4());
    live.clock.shift(7 * DAY);
    assert_eq!(
        contract_error(live.sign(&client, &submitted)),
        ContractWorkflowError::Expired
    );
    let view = live.service(&client).request(TOKEN, live.request).unwrap();
    assert_eq!(
        (view.status, view.contract.state),
        (RequestStatus::Expired, ContractState::Draft)
    );
    assert_eq!(closer(&live).run_once().unwrap(), 1);
    assert_eq!(closer(&live).run_once().unwrap(), 0);
    let row = live
        .test
        .db()
        .admin
        .query_one(
            "SELECT actor_id IS NULL,actor_label FROM contract_events WHERE kind='expired'",
            &[],
        )
        .unwrap();
    assert!(row.get::<_, bool>(0));
    assert_eq!(row.get::<_, String>(1), EXPIRY_ACTOR);
    // requested: the Client; expired: the Client and the firm signer.
    assert_eq!(live.count("contract_notifications"), 3);
    assert!(opens(&live.test));
}

#[test]
fn a_new_request_first_expires_an_exhausted_one_in_the_same_transaction() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    live.clock.shift(7 * DAY);
    let firm = live.firm();
    live.request = Uuid::new_v4();
    let receipt = live
        .service(&firm)
        .create_request(
            TOKEN,
            live.test.individual.db.case,
            live.contract,
            &live.new_request(2),
        )
        .unwrap();
    assert_eq!(receipt.event.revision, 4);
    let kinds: Vec<String> = live
        .test
        .db()
        .admin
        .query("SELECT kind FROM contract_events ORDER BY revision", &[])
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(kinds, ["created", "requested", "expired", "requested"]);
    assert!(opens(&live.test));
}

#[test]
fn closings_follow_their_authors_and_turns() {
    let Some(live) = Live::new() else { return };
    let live = live.requested();
    let (client, firm) = (live.client(), live.firm());
    let decline = live.service(&firm).close(
        TOKEN,
        live.request,
        Uuid::new_v4(),
        2,
        CloseOutcome::Decline,
        None,
    );
    assert_eq!(contract_error(decline), ContractWorkflowError::OutOfTurn);
    let stale = live.service(&client).close(
        TOKEN,
        live.request,
        Uuid::new_v4(),
        1,
        CloseOutcome::Reject,
        None,
    );
    assert_eq!(
        contract_error(stale),
        ContractWorkflowError::RevisionConflict
    );
    let rejected = live
        .service(&client)
        .close(
            TOKEN,
            live.request,
            Uuid::new_v4(),
            2,
            CloseOutcome::Reject,
            Some("No acepto"),
        )
        .unwrap();
    assert_eq!(rejected.event.kind, EventKind::Rejected);
    let cancel = live.service(&firm).close(
        TOKEN,
        live.request,
        Uuid::new_v4(),
        3,
        CloseOutcome::Cancel,
        Some("Ya no aplica"),
    );
    assert_eq!(contract_error(cancel), ContractWorkflowError::RequestClosed);
    let abandoned = live
        .service(&firm)
        .abandon(
            TOKEN,
            live.test.individual.db.case,
            live.contract,
            Uuid::new_v4(),
            3,
            "Otro instrumento",
        )
        .unwrap();
    assert_eq!(abandoned.event.kind, EventKind::Abandoned);
    assert!(opens(&live.test));
}
