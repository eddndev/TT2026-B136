use super::*;
use application::{
    measure_corrections::MeasureRecordHistoryEvidence,
    precautionary_measures::{MeasureDecisionRecordHistoryEvidence, MeasureHistoryEvidence},
};
use time::{format_description::well_known::Rfc3339, Duration};

pub fn empty_history() -> MeasureDecisionRecordHistoryEvidence {
    MeasureDecisionRecordHistoryEvidence {
        records: MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence { groups: vec![] },
            administrative: vec![],
        },
        decisions: vec![],
    }
}
pub fn stored(
    captures: Vec<PrecautionaryHearingCapture>,
) -> PrecautionaryHearingRecordStoredOperation {
    let evidence = empty_history();
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &captures[0], &evidence)
            .unwrap();
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher, &captures, &origin, &evidence,
    )
    .unwrap();
    PrecautionaryHearingRecordStoredOperation {
        capture: captures.last().unwrap().clone(),
        history: PrecautionaryHearingRecordHistoryEvidence {
            origin,
            captures,
            record_history: evidence,
        },
    }
}
pub fn initial() -> PrecautionaryHearingRecordStoredOperation {
    stored(vec![crate::receipt_support::scheduled()])
}
pub fn replaced() -> PrecautionaryHearingRecordStoredOperation {
    let first = crate::receipt_support::scheduled();
    let second = crate::receipt_support::Fixture::replace(&first).capture(
        Some(&first),
        crate::receipt_support::at() + Duration::seconds(2),
    );
    stored(vec![first, second])
}
pub fn cancelled() -> PrecautionaryHearingRecordStoredOperation {
    let mut prior = replaced();
    let next = crate::receipt_support::Fixture::cancel(&prior.capture).capture(
        Some(&prior.capture),
        crate::receipt_support::at() + Duration::seconds(3),
    );
    prior.history.captures.push(next);
    stored(prior.history.captures)
}
pub fn values(value: &PrecautionaryHearingValues) -> Value {
    let basis = value.scheduling_basis();
    let support = basis.support();
    json!({"purpose":value.purpose().as_str(),"scheduled_at":value.scheduled_at().value().format(&Rfc3339).unwrap(),
        "modality":value.modality().as_str(),"venue":value.venue().as_str(),"note":value.note().map(|v|v.as_str()),
        "participants":value.participants().iter().map(|p|json!({"participant_id":p.id().to_string(),"revision":p.revision().get()})).collect::<Vec<_>>(),
        "scheduling_basis":{"statement":basis.statement().as_str(),"locator":basis.locator().as_str(),
            "support":{"document_id":support.reference().id.to_string(),"version":support.reference().version.get(),"digest":support.digest().to_hex()}},
        "review_targets":value.review_targets().iter().map(|r|json!({"id":r.id().to_string(),"revision":r.revision().get(),"capture_digest":r.digest().to_hex()})).collect::<Vec<_>>()})
}
pub fn expectation(value: PrecautionaryContextExpectation) -> Value {
    json!({"administration_revision":value.administration_revision.get(),"stage_revision":value.stage_revision.get(),"context_digest":value.context_digest.to_hex()})
}
pub fn command(value: &PrecautionaryHearingCommand) -> Value {
    let change = match &value.change {
        PrecautionaryHearingChange::Schedule { context, values: v } => {
            json!({"action":"schedule","context":expectation(*context),"values":values(v)})
        }
        PrecautionaryHearingChange::Replace {
            expected_revision,
            expected_capture_digest,
            context,
            values: v,
            reason,
        } => json!({
            "action":"replace","expected_revision":expected_revision.get(),"expected_capture_digest":expected_capture_digest.to_hex(),
            "context":expectation(*context),"values":values(v),"reason":reason.as_str()}),
        PrecautionaryHearingChange::Cancel {
            expected_revision,
            expected_capture_digest,
            reason,
        } => json!({
            "action":"cancel","expected_revision":expected_revision.get(),"expected_capture_digest":expected_capture_digest.to_hex(),"reason":reason.as_str()}),
    };
    json!({"case_id":case_id().to_string(),"operation_id":value.operation_id.to_string(),"hearing_id":value.hearing_id.to_string(),"change":change})
}
pub fn body() -> Value {
    command(&initial().capture.review.command)
}
pub fn submission(row: &PrecautionaryHearingRecordStoredOperation) -> Value {
    json!({"command":command(&row.capture.review.command),"expected_submission_digest":row.capture.review.submission_digest.to_hex(),"expected_review_digest":row.capture.review.review_digest.to_hex()})
}

pub fn review_operation() -> PrecautionaryHearingRecordStoredOperation {
    use application::precautionary_measures::{measure_group_origin, MeasureGroupEvidence};
    let group = crate::measure_decision_fixtures::Fixture::single().capture();
    let reference = crate::measure_decision_fixtures::reference(&group.measures[0]);
    let mut history = empty_history();
    let origin = measure_group_origin(&Hasher, &group, &history.records.judicial).unwrap();
    history.records.judicial.groups.push(MeasureGroupEvidence {
        origin,
        capture: group,
    });
    let mut fixture = crate::receipt_support::Fixture::schedule();
    let mut input = crate::receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![reference];
    fixture.command.change = PrecautionaryHearingChange::Schedule {
        context: crate::receipt_support::expectation(&fixture.context),
        values: PrecautionaryHearingValues::new(input).unwrap(),
    };
    let capture = prepare_precautionary_hearing_with_decision_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        PrecautionaryHearingDecisionPreparationMaterial {
            observed_context: fixture.context,
            sources: fixture.sources,
            predecessor: None,
            decision_history: &history,
        },
    )
    .unwrap()
    .into_capture(&Hasher, crate::receipt_support::at())
    .unwrap();
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &capture, &history).unwrap();
    PrecautionaryHearingRecordStoredOperation {
        capture: capture.clone(),
        history: PrecautionaryHearingRecordHistoryEvidence {
            origin,
            captures: vec![capture],
            record_history: history,
        },
    }
}
