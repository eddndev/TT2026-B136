use crate::hearing_derived_deadline_support::Fixture;
use crate::hearing_result_support as results;
use application::{
    deadline_reevaluation::{DependencyFamily, SourceEventReference},
    hearing_results::*,
};
use time::OffsetDateTime;

/// A separately recorded source fixture, constructed after prospective review.
/// It exercises capture validation without claiming a database write occurred.
pub fn recorded_source(fixture: &Fixture, recorded_at: OffsetDateTime) -> HearingResultDetail {
    let mut source = results::detail(
        fixture.actor.id,
        &fixture.command.result,
        &fixture.result_preparation,
    );
    source.snapshot.recorded_at = recorded_at;
    source.snapshot.recorded_by.email = fixture.actor.email.clone();
    hearing_result_receipt_matches(results::hasher().as_ref(), &source).unwrap();
    source
}

pub fn source_event(source: &HearingResultDetail) -> SourceEventReference {
    let source = &source.snapshot;
    SourceEventReference {
        sequence: 7,
        family: DependencyFamily::HearingResult,
        source_id: source.id.as_uuid(),
        revision: source.revision.get(),
        case_id: Some(source.case_id),
        hearing_id: Some(source.hearing_id.as_uuid()),
        operation_id: source.receipt.operation_id.as_uuid(),
    }
}

/// Keep alternative source envelopes valid so capture tests isolate draft mismatch.
pub fn resign_recorded_source(source: &mut HearingResultDetail) {
    let snapshot = &mut source.snapshot;
    let command = HearingResultCommand {
        operation_id: snapshot.receipt.operation_id,
        hearing_id: snapshot.hearing_id,
        result_id: snapshot.id,
        change: HearingResultChange::Record {
            anchor_revision: snapshot.anchor.revision,
            continuation: None,
            values: snapshot.values.clone(),
        },
    };
    snapshot.values_digest =
        hearing_result_values_digest(results::hasher().as_ref(), &snapshot.values);
    snapshot.receipt.submission_digest = hearing_result_submission_digest(
        results::hasher().as_ref(),
        snapshot.recorded_by.id,
        snapshot.case_id,
        &command,
        &snapshot.anchor,
        snapshot.continuation.as_ref(),
        snapshot.values_digest,
    );
    hearing_result_receipt_matches(results::hasher().as_ref(), source).unwrap();
}
