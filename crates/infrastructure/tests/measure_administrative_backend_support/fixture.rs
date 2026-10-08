pub use crate::measure_fixture::{
    note, processor, FixedClock, Fixture, FormatCheck, Seed, TestIdentity,
};
pub use application::measure_corrections::*;
pub use application::precautionary_measures::*;
pub use application::{
    identity::Principal, precautionary_hearings::PrecautionaryContextExpectation,
};
pub use domain::precautionary_hearings::PrecautionaryMeasureRef;
pub use domain::precautionary_measures::*;
pub use infrastructure::{PostgresMeasureAdministrativeStore, RingSha256Hasher};
pub use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresMeasureAdministrativeStore> {
    Arc::new(
        PostgresMeasureAdministrativeStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}

pub fn service(db: &Fixture, actor: Principal) -> MeasureAdministrativeService {
    service_with_format(db, actor, FormatCheck(None))
}

pub fn service_with_format(
    db: &Fixture,
    actor: Principal,
    format: FormatCheck,
) -> MeasureAdministrativeService {
    MeasureAdministrativeService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(format),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn reference(capture: &MeasureCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

pub fn corrected_reference(capture: &MeasureAdministrativeCapture) -> PrecautionaryMeasureRef {
    let row = &capture.records[0];
    PrecautionaryMeasureRef::new(row.result.id, row.result.revision, row.capture_digest)
}

pub fn correction(
    target: PrecautionaryMeasureRef,
    context: PrecautionaryContextExpectation,
    values: &MeasureValues,
    conditions: &str,
) -> MeasureAdministrativeCommand {
    let supervision = match values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement.clone(),
        MeasureSupervision::Unknown { reason } => reason.clone(),
    };
    MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::new(),
        target,
        context,
        reason: note("Correct the declared transcription"),
        action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            note(conditions),
            values.validity().clone(),
            supervision,
        )),
    }
}

pub fn confirmation(review: &MeasureAdministrativeReview) -> MeasureAdministrativeConfirmation {
    MeasureAdministrativeConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

pub fn persist(
    db: &Fixture,
    actor: Principal,
    command: MeasureAdministrativeCommand,
) -> MeasureAdministrativeStoredOperation {
    let workflow = service(db, actor);
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    workflow
        .submit("session", db.case, command, confirmation(&review))
        .unwrap()
}

pub fn setup(
    db: &mut Fixture,
) -> (
    Seed,
    MeasureDecisionStoredOperation,
    MeasureAdministrativeCommand,
) {
    let seed = crate::measure_fixture::setup(db);
    let judicial = crate::measure_fixture::persist(db, seed.actor.clone(), seed.command.clone());
    let measure = &judicial.group.measures[0];
    let command = correction(
        reference(measure),
        seed.command.context,
        &measure.result.values,
        "Corrected reporting terms",
    );
    (seed, judicial, command)
}

pub fn mark(
    target: PrecautionaryMeasureRef,
    context: PrecautionaryContextExpectation,
) -> MeasureAdministrativeCommand {
    MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::new(),
        target,
        context,
        reason: note("The declared record was entered in error"),
        action: MeasureAdministrativeAction::MarkEnteredInError,
    }
}

pub fn effect_command(
    base: &MeasureDecisionCommand,
    effects: Vec<MeasureEffect>,
) -> MeasureDecisionCommand {
    let mut command = crate::measure_fixture::fresh(base);
    command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    command
}

pub fn assert_retained(
    administrative: &MeasureAdministrativeStoredOperation,
    judicial: &MeasureDecisionStoredOperation,
    previous: &MeasureCapture,
) {
    let result = &administrative.capture.review.result;
    assert_eq!(
        result.record_root,
        MeasureRecordRoot::Judicial(previous.result.origin)
    );
    assert_eq!(result.judicial_origin, previous.result.origin);
    assert_eq!(result.last_action, previous.result.action);
    assert_eq!(result.last_judicial.reference, reference(previous));
    assert_eq!(
        result.last_judicial.owner,
        MeasureGroupRef {
            operation_id: judicial.origin.operation_id,
            decision_id: judicial.origin.decision_id,
            group_digest: judicial.group.capture_digest,
        }
    );
    assert_eq!(result.sources, previous.result.sources);
    assert_eq!(result.projection, previous.result.projection);
    assert_eq!(result.values.subject(), previous.result.values.subject());
    assert_eq!(result.values.kind(), previous.result.values.kind());
    assert_eq!(
        administrative.capture.review.support,
        judicial.group.decision.support
    );
    let target = administrative
        .capture
        .records
        .iter()
        .find(|row| row.result.id == result.id)
        .expect("the original target must remain in the captured owner");
    assert_eq!(target.result, *result);
}

pub fn same_operation(
    actual: &MeasureAdministrativeStoredOperation,
    expected: &MeasureAdministrativeStoredOperation,
) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    for operation in [&mut actual, &mut expected] {
        operation
            .record_history
            .records
            .judicial
            .groups
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
        operation
            .record_history
            .records
            .administrative
            .sort_by_key(|a| a.origin.operation_id.as_uuid());
        operation
            .record_history
            .decisions
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
    }
    assert_eq!(actual, expected);
}

pub fn reopened(db: &Fixture, actor: &Principal, original: &MeasureAdministrativeStoredOperation) {
    let bytes = measure_administrative_capture_bytes(&original.capture).unwrap();
    let workflow = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("original administrative replay must skip support admission")
        }))),
    );
    let command = original.capture.review.command.clone();
    assert_eq!(
        workflow
            .prepare("session", db.case, command.clone())
            .unwrap(),
        original.capture.review
    );
    let actual = workflow
        .submit(
            "session",
            db.case,
            command,
            confirmation(&original.capture.review),
        )
        .unwrap();
    same_operation(&actual, original);
    assert_eq!(
        measure_administrative_capture_bytes(&actual.capture).unwrap(),
        bytes
    );
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one(
        "SELECT jsonb_build_object(
         'operations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY operation_id) FROM case_measure_operations r),
         'decisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY operation_id) FROM case_measure_decisions r),
         'administrations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY operation_id) FROM case_measure_administrations r),
         'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_measures r),
         'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY measure_id,revision) FROM case_measure_revisions r),
         'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))",
        &[],
    ).unwrap().get(0)
}
