use application::case_stages::CaseStageEntry;
use application::precautionary_hearings::*;
use application::typed_participants::ParticipantRevisionSnapshot;
use domain::cases::CaseId;
use domain::crypto::DocumentHasher;
use domain::hearings::{HearingNote, HearingStatus};
use domain::precautionary_hearings::{
    PrecautionaryHearingOperationId, PrecautionaryHearingRevision,
};
use time::Duration;
use uuid::Uuid;

use crate::precautionary_receipt_support::{at, expectation, scheduled, Fixture, Hasher};

pub fn operation(value: u128) -> PrecautionaryHearingOperationId {
    PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(value))
}

pub fn replacement(
    previous: &PrecautionaryHearingCapture,
    operation_id: u128,
) -> PrecautionaryHearingCapture {
    let mut fixture = Fixture::replace(previous);
    fixture.command.operation_id = operation(operation_id);
    fixture.capture(Some(previous), previous.recorded_at + Duration::seconds(2))
}

pub fn complete_chain() -> Vec<PrecautionaryHearingCapture> {
    let initial = scheduled();
    let replaced = replacement(&initial, 31);
    let cancelled =
        Fixture::cancel(&replaced).capture(Some(&replaced), at() + Duration::seconds(4));
    vec![initial, replaced, cancelled]
}

pub fn rehash(capture: &mut PrecautionaryHearingCapture) {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &review.actor,
            review.case_id,
            &review.command,
            &review.resolved_values,
        )
        .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&precautionary_hearing_review_bytes(review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}

pub fn remap_case(capture: &mut PrecautionaryHearingCapture, case_id: CaseId) {
    capture.review.case_id = case_id;
    for context in [
        &mut capture.review.scheduling_context,
        &mut capture.review.observed_context,
    ] {
        let mut material = context.material().clone();
        material.case_id = case_id;
        material.administration.case_id = case_id;
        material.stage_administration.case_id = case_id;
        match &mut material.stage {
            CaseStageEntry::Initial(stage) => stage.case_id = case_id,
            CaseStageEntry::Changed(stage) => stage.case_id = case_id,
        }
        *context = PrecautionaryContext::new(&Hasher, material).unwrap();
    }
    match &mut capture.review.command.change {
        PrecautionaryHearingChange::Schedule { context, .. }
        | PrecautionaryHearingChange::Replace { context, .. } => {
            *context = expectation(&capture.review.observed_context);
        }
        PrecautionaryHearingChange::Cancel { .. } => {}
    }
    for detail in &mut capture.review.sources.participants {
        match &mut detail.revision {
            ParticipantRevisionSnapshot::Manual(participant) => participant.case_id = case_id,
            ParticipantRevisionSnapshot::Typed(participant) => participant.case_id = case_id,
        }
        if let Some(subject) = &mut detail.bound_subject {
            subject.case_id = case_id;
        }
    }
    for participant in &mut capture.review.participants {
        participant.snapshot.case_id = case_id;
        participant.overview.case_id = case_id;
    }
    rehash(capture);
}

pub fn after_cancel(
    cancelled: &PrecautionaryHearingCapture,
    replace: bool,
) -> PrecautionaryHearingCapture {
    let mut candidate = cancelled.clone();
    let previous = &cancelled.review;
    candidate.review.command.operation_id = operation(33);
    candidate.review.result_revision =
        PrecautionaryHearingRevision::new(previous.result_revision.get() + 1).unwrap();
    let reason = HearingNote::new("Recorded subsequent instruction").unwrap();
    candidate.review.command.change = if replace {
        candidate.review.status = HearingStatus::Scheduled;
        candidate.review.scheduling_context = candidate.review.observed_context.clone();
        PrecautionaryHearingChange::Replace {
            expected_revision: previous.result_revision,
            expected_capture_digest: cancelled.capture_digest,
            context: expectation(&candidate.review.observed_context),
            values: previous.resolved_values.clone(),
            reason,
        }
    } else {
        PrecautionaryHearingChange::Cancel {
            expected_revision: previous.result_revision,
            expected_capture_digest: cancelled.capture_digest,
            reason,
        }
    };
    candidate.recorded_at += Duration::seconds(2);
    rehash(&mut candidate);
    candidate
}

pub fn assert_flat_and_adjacent(captures: &[PrecautionaryHearingCapture]) {
    for capture in captures {
        precautionary_hearing_receipt_matches(&Hasher, capture).unwrap();
    }
    for pair in captures.windows(2) {
        precautionary_hearing_transition_matches(&Hasher, &pair[0], &pair[1]).unwrap();
    }
}
