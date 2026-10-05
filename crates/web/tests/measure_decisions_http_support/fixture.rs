use super::*;
use crate::{measure_decision_fixtures::Fixture, record_decision_support::FixtureV2};

pub fn g1() -> MeasureDecisionRecordReceipt {
    from_g1(Fixture::single())
}
pub fn from_g1(fixture: Fixture) -> MeasureDecisionRecordReceipt {
    let group = fixture.capture();
    let measure_history = MeasureHistoryEvidence { groups: vec![] };
    let origin = measure_group_origin(&Hasher, &group, &measure_history).unwrap();
    MeasureDecisionRecordReceipt::V1(Box::new(MeasureDecisionStoredOperation {
        group,
        origin,
        measure_history,
    }))
}
pub fn g2() -> MeasureDecisionRecordReceipt {
    let prior = crate::record_support::RecordFixture::initial();
    from_g2(FixtureV2::confirm(&prior.capture(), &prior.history))
}
pub fn from_g2(fixture: FixtureV2) -> MeasureDecisionRecordReceipt {
    let group = fixture.capture();
    let origin = measure_group_origin_v2(&Hasher, &group, &fixture.history).unwrap();
    MeasureDecisionRecordReceipt::V2(Box::new(MeasureDecisionRecordStoredOperation {
        group,
        origin,
        record_history: fixture.history,
    }))
}
pub fn review(row: &MeasureDecisionRecordReceipt) -> MeasureDecisionRecordReview {
    match row {
        MeasureDecisionRecordReceipt::V1(v) => {
            MeasureDecisionRecordReview::V1(Box::new(v.group.review.clone()))
        }
        MeasureDecisionRecordReceipt::V2(v) => {
            MeasureDecisionRecordReview::V2(Box::new(v.group.review.clone()))
        }
    }
}
pub fn submission(row: &MeasureDecisionRecordReceipt) -> Value {
    json!({"command":command(row.command()),
        "expected_submission_digest":row.submission_digest().to_hex(),
        "expected_review_digest":row.review_digest().to_hex()})
}
