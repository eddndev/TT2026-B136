use super::*;
use application::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor},
    identity::IdentityWorkflow,
    measure_corrections::{MeasureCaptureValidity, MeasureRecordRoot, OwnedJudicialMeasure},
};
use domain::{
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Role, UserId},
    precautionary_hearings::MeasureId,
    precautionary_measures::*,
};
use std::sync::{Arc, Mutex};

#[path = "supplemental_admission.rs"]
mod admission;
#[path = "supplemental_authorization.rs"]
mod authorization;
#[path = "supplemental_clock.rs"]
mod clock;
#[path = "supplemental_effects.rs"]
mod effects;
#[path = "supplemental_evidence.rs"]
mod evidence;
#[path = "supplemental_replay.rs"]
mod replay;

fn from_pure(mut value: crate::record_decision_support::FixtureV2, document: u128) -> Fixture {
    let record = crate::crypto::processor()
        .prepare_version(
            DocumentId::from_uuid(Uuid::from_u128(document)),
            DocumentVersion::initial(),
            "later-record-decision.pdf",
            b"Exact later decision support",
        )
        .unwrap();
    let mut input = crate::measure_decision_fixtures::decision_input(&value.command.values);
    input.support = HearingSupportRef::new(
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
    );
    value.command.values = MeasureDecisionValues::new(input);
    Fixture {
        actor: value.actor,
        case_id: value.case_id,
        command: value.command,
        material: MeasureDecisionRecordReady {
            context: value.material.context,
            support_record: record,
            anchor: value.material.anchor,
            predecessors: value.material.predecessors,
            result_sources: value.material.result_sources,
            record_history: value.history,
        },
    }
}

fn initial() -> Fixture {
    from_pure(
        crate::record_decision_support::FixtureV2::initial(
            crate::measure_decision_fixtures::Fixture::single(),
        ),
        93,
    )
}

fn next(prior: &MeasureDecisionRecordStoredOperation) -> Fixture {
    from_pure(
        crate::record_decision_support::FixtureV2::next(&prior.group, &prior.record_history, 2),
        94,
    )
}

fn submit(fixture: Fixture) -> MeasureDecisionRecordStoredOperation {
    let expected = confirmation(&fixture.review());
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now()));
    let harness = harness(store, identity(fixture.actor));
    let receipt = harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .unwrap();
    let MeasureDecisionRecordReceipt::V2(operation) = receipt else {
        panic!("expected V2")
    };
    measure_decision_group_v2_matches(&Hasher, &operation.group, &operation.record_history)
        .unwrap();
    *operation
}

fn replay_store(receipt: MeasureDecisionRecordReceipt) -> MockStore {
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| {
            Ok(MeasureDecisionRecordPreparation::Replay(Box::new(receipt)))
        });
    store
}

fn custom_service(
    store: MockStore,
    identity: impl IdentityWorkflow + 'static,
    validator: Arc<dyn DocumentFormatBatchValidator>,
    clock: Arc<dyn Clock + Send + Sync>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
) -> (
    MeasureDecisionRecordService,
    Arc<Mutex<crate::observed_crypto::Observations>>,
) {
    let (processor, observations): (DocumentProcessor, _) = crate::observed_crypto::processor();
    (
        MeasureDecisionRecordService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(processor),
            validator,
            hasher,
            clock,
        ),
        observations,
    )
}
