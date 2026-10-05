#![allow(dead_code, unused_imports)]

mod authorization;
mod bounds;
mod clocks;
mod evidence;
mod lifecycle;
mod pagination;
mod ports;
mod sources;
pub use crate::context_support::Hasher;
pub use crate::decision_review_support::{judicial_fixture, DecisionReviewFixture};
pub use crate::record_decision_support::{append_v2, reference_v2, FixtureV2};
pub use application::{
    identity::Principal, precautionary_hearings::*, precautionary_measures::*, ApplicationError,
};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
    identity::{Role, UserId},
    precautionary_hearings::*,
};
pub use ports::*;
pub use std::sync::{Arc, Mutex};

pub fn id(serial: u128) -> PrecautionaryHearingId {
    PrecautionaryHearingId::from_uuid(uuid::Uuid::from_u128(serial))
}
pub fn at() -> OffsetDateTime {
    crate::precautionary_receipt_support::at()
}
pub fn now() -> OffsetDateTime {
    at() + time::Duration::seconds(100)
}
pub fn reader(role: Role) -> Principal {
    Principal {
        id: UserId::from_uuid(uuid::Uuid::from_u128(900)),
        email: "current-reader@example.test".into(),
        role,
    }
}

pub fn operation(serial: u128) -> PrecautionaryHearingRecordStoredOperation {
    let fixture = judicial_fixture();
    let group = fixture.capture();
    let history = append_v2(&fixture.history, &group);
    scheduled(
        serial,
        vec![reference_v2(&group.measures[0])],
        history,
        group.recorded_at,
    )
}

pub fn scheduled(
    serial: u128,
    targets: Vec<PrecautionaryMeasureRef>,
    history: MeasureDecisionRecordHistoryEvidence,
    recorded_at: OffsetDateTime,
) -> PrecautionaryHearingRecordStoredOperation {
    let mut fixture = DecisionReviewFixture::schedule(targets, history.clone());
    fixture.hearing.command.hearing_id = id(serial);
    fixture.hearing.command.operation_id =
        PrecautionaryHearingOperationId::from_uuid(uuid::Uuid::from_u128(7000 + serial));
    let capture = fixture.capture(None, recorded_at);
    let origin =
        precautionary_hearing_origin_with_decision_history(&Hasher, &capture, &history).unwrap();
    PrecautionaryHearingRecordStoredOperation {
        history: PrecautionaryHearingRecordHistoryEvidence {
            origin,
            captures: vec![capture.clone()],
            record_history: history,
        },
        capture,
    }
}

pub fn cancelled(
    previous: &PrecautionaryHearingRecordStoredOperation,
) -> PrecautionaryHearingRecordStoredOperation {
    let mut fixture =
        DecisionReviewFixture::cancel(&previous.capture, previous.history.record_history.clone());
    fixture.hearing.command.hearing_id = previous.capture.review.command.hearing_id;
    fixture.hearing.command.operation_id = PrecautionaryHearingOperationId::new();
    let capture = fixture.capture(
        Some(&previous.capture),
        previous.capture.recorded_at + time::Duration::seconds(1),
    );
    let mut history = previous.history.clone();
    history.captures.push(capture.clone());
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &history.captures,
        &history.origin,
        &history.record_history,
    )
    .unwrap();
    PrecautionaryHearingRecordStoredOperation { capture, history }
}

pub fn page(
    case_id: CaseId,
    items: Vec<PrecautionaryHearingRecordStoredOperation>,
) -> PrecautionaryHearingRecordPage {
    PrecautionaryHearingRecordPage {
        case_id,
        items,
        has_more: false,
        next_after_id: None,
    }
}

pub fn refresh(capture: &mut PrecautionaryHearingCapture) {
    use domain::crypto::DocumentHasher;
    capture.review.submission_digest = Hasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &capture.review.actor,
            capture.review.case_id,
            &capture.review.command,
            &capture.review.resolved_values,
        )
        .unwrap(),
    );
    capture.review.review_digest =
        Hasher.hash_bytes(&precautionary_hearing_review_bytes(&capture.review).unwrap());
    capture.capture_digest =
        Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture).unwrap());
}
