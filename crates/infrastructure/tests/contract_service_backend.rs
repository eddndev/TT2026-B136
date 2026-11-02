//! The bilateral contract workflow end to end over PostgreSQL, real keys,
//! CRLs and a local time-stamping authority, checked by the evidence
//! validator: docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_service_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::contracts::*;
use contract_service_support::{contract_error, Live};
use contract_storage_support::opens;
use domain::clock::Clock as _;
use evidence_verify::{
    policy::{PinnedRoot, Policy, Purposes},
    report::Profile,
    validate, Outcome,
};
use signing_binding_support::TOKEN;
use uuid::Uuid;

fn policy(live: &Live) -> Policy {
    let root = individual_signing_fixture::trust().inspection.root_der;
    Policy::new(
        vec![PinnedRoot {
            sha256: evidence_verify::certificate::sha256(&root),
            purposes: Purposes::ALL,
            label: "credential trust root".to_owned(),
        }],
        live.clock.now().unix_timestamp() + 60,
    )
}

fn audits(live: &mut Live, action: &str) -> i64 {
    live.test
        .db()
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action=$1",
            &[&action],
        )
        .unwrap()
        .get(0)
}

#[test]
fn both_parties_sign_and_each_package_passes_the_validator() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let (client, firm) = (live.client(), live.firm());
    let view = live.service(&client).request(TOKEN, live.request).unwrap();
    assert_eq!(view.status, RequestStatus::AwaitingClient);
    assert_eq!(view.contract.state, ContractState::Signing);
    assert_eq!(live.service(&client).my_requests(TOKEN).unwrap().len(), 1);

    let content = live.service(&client).content(TOKEN, live.request).unwrap();
    assert_eq!(&content.bytes[..], individual_signing_fixture::PDF);
    assert_eq!(audits(&mut live, "contract.content_read"), 1);

    let first = live.signed(&client, Uuid::new_v4());
    let receipt = live.sign(&client, &first).unwrap();
    assert_eq!(receipt.event.kind, EventKind::ClientSigned);
    assert_eq!(live.sign(&client, &first).unwrap(), receipt, "exact replay");
    assert!(opens(&live.test));

    let partial = live
        .service(&client)
        .export(TOKEN, live.request, ContractStage::Partial)
        .unwrap();
    let report = validate(&partial.archive, &policy(&live));
    assert_eq!(
        report.profile,
        Some(Profile::BilateralContractPartialV1),
        "{report:#?}"
    );
    assert_eq!(report.overall, Outcome::Indeterminate, "{report:#?}");
    assert_eq!(
        contract_error(
            live.service(&client)
                .export(TOKEN, live.request, ContractStage::Final)
        ),
        ContractWorkflowError::NotComplete
    );

    let second = live.signed(&firm, Uuid::new_v4());
    let completed = live.sign(&firm, &second).unwrap();
    assert_eq!(completed.event.kind, EventKind::Completed);
    assert!(completed.manifest_sha256.is_some());
    assert_eq!(
        live.sign(&firm, &second).unwrap(),
        completed,
        "exact replay"
    );
    assert!(
        opens(&live.test),
        "the startup inventory admits the completion"
    );

    let package = live
        .service(&client)
        .export(TOKEN, live.request, ContractStage::Final)
        .unwrap();
    let report = validate(&package.archive, &policy(&live));
    assert_eq!(
        report.profile,
        Some(Profile::BilateralContractV1),
        "{report:#?}"
    );
    assert_eq!(report.overall, Outcome::Checked, "{report:#?}");
    assert_eq!(audits(&mut live, "contract.evidence_exported"), 2);

    let view = live
        .service(&firm)
        .contract(TOKEN, live.test.individual.db.case, live.contract)
        .unwrap();
    assert_eq!(view.state, ContractState::Signed);
    assert_eq!(view.snapshot.revision(), 4);
    let receipt = live
        .service(&firm)
        .request_receipt(TOKEN, live.request, second.operation)
        .unwrap();
    assert_eq!(receipt, completed);
    // requested: the Client; client_signed and completed: Client and firm signer,
    // who is also the requester.
    assert_eq!(live.count("contract_notifications"), 5);
    assert!(live.service(&client).content(TOKEN, live.request).is_ok());
    assert_eq!(
        contract_error(live.service(&firm).content(TOKEN, live.request).map(|_| ())),
        ContractWorkflowError::NotFound,
        "the firm signer reads the request route only while it is open"
    );
}

#[test]
fn a_different_signature_for_the_same_operation_conflicts() {
    let Some(live) = Live::new() else { return };
    let live = live.requested();
    let client = live.client();
    let first = live.signed(&client, Uuid::new_v4());
    live.sign(&client, &first).unwrap();
    let mut changed = first.clone();
    changed.signature[0] ^= 1;
    match live.sign(&client, &changed) {
        Err(application::ApplicationError::DocumentSigning(error)) => assert_eq!(
            error,
            application::document_signing::SigningWorkflowError::Conflict
        ),
        other => panic!("expected signing_conflict, got {other:?}"),
    }
}

#[test]
fn an_unassigned_litigator_cannot_tell_documents_or_contracts_apart() {
    let Some(mut live) = Live::new() else { return };
    let id = live.test.db().user("litigator", false);
    let outsider = signing_binding_support::principal(live.test.db(), id);
    let case = live.test.individual.db.case;
    let document = live.test.individual.record.id;
    let create = |live: &Live, document| {
        live.service(&outsider)
            .create(TOKEN, case, Uuid::new_v4(), document, "Contrato ajeno")
    };
    let missing = domain::crypto::DocumentId::from_uuid(Uuid::new_v4());
    assert_eq!(
        contract_error(create(&live, missing)),
        ContractWorkflowError::NotFound
    );
    assert_eq!(
        contract_error(create(&live, document)),
        ContractWorkflowError::NotFound
    );
    let mut live = live.requested();
    assert_eq!(
        contract_error(create(&live, document)),
        ContractWorkflowError::NotFound
    );
    assert_eq!(live.count("contracts"), 1);
    assert_eq!(audits(&mut live, "contract.created"), 1);
}

#[test]
fn staff_without_a_designation_cannot_sign_and_strangers_learn_nothing() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let firm = live.firm();
    assert!(matches!(
        live.service(&firm)
            .prepare(TOKEN, live.request, Uuid::new_v4()),
        Err(application::ApplicationError::Contract(
            ContractWorkflowError::OutOfTurn
        ))
    ));
    let id = live.test.db().user("client", false);
    let stranger = signing_binding_support::principal(live.test.db(), id);
    assert_eq!(
        contract_error(live.service(&stranger).request(TOKEN, live.request)),
        ContractWorkflowError::NotFound
    );
}
