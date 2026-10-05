use crate::decision_review_support::*;
use crate::record_review_support::RecordReviewFixture;

#[test]
fn record_history_and_decision_history_proofs_preserve_identical_hearing_bytes() {
    let administrative = RecordFixture::initial();
    let correction = administrative.capture();
    let records = append_administrative(&administrative.history, &correction);
    let selected = record_reference(&correction.records[0]);
    let old_fixture = RecordReviewFixture::schedule(vec![selected], records.clone());
    let expected = old_fixture.capture(None, correction.recorded_at);
    let fixture = DecisionReviewFixture {
        hearing: old_fixture.hearing,
        decision_history: MeasureDecisionRecordHistoryEvidence {
            records,
            decisions: vec![],
        },
    };
    let actual = fixture.capture(None, correction.recorded_at);
    assert_eq!(actual, expected);
    assert_eq!(
        precautionary_hearing_capture_bytes(&actual).unwrap(),
        precautionary_hearing_capture_bytes(&expected).unwrap()
    );
    assert_eq!(
        precautionary_hearing_origin_with_decision_history(
            &Hasher,
            &actual,
            &fixture.decision_history
        )
        .unwrap(),
        precautionary_hearing_origin_with_record_history(
            &Hasher,
            &expected,
            &fixture.decision_history.records
        )
        .unwrap()
    );
}

#[test]
fn imposition_with_empty_versioned_decision_evidence_keeps_its_original_capture() {
    let hearing = HearingFixture::schedule();
    let at = crate::precautionary_receipt_support::at();
    let expected = hearing.clone().capture(None, at);
    let fixture = DecisionReviewFixture {
        hearing,
        decision_history: empty_decision_history(),
    };
    let actual = fixture.capture(None, at);
    assert_eq!(actual, expected);
    precautionary_hearing_receipt_with_decision_history_matches(
        &Hasher,
        &actual,
        &fixture.decision_history,
    )
    .unwrap();
}

#[test]
fn external_owner_order_does_not_change_a_review_of_the_final_post_m2_correction() {
    let judicial = judicial_fixture();
    let first_group = judicial.capture();
    let next_fixture = FixtureV2::next(&first_group, &judicial.history, 2);
    let next_group = next_fixture.capture();
    let administrative = AdministrativeFixture::after(&next_group, &next_fixture.history, 1);
    let correction = administrative.capture();
    let history = append_administrative_decision_history(&administrative.history, &correction);
    let mut fixture =
        DecisionReviewFixture::schedule(vec![record_reference(&correction.records[0])], history);
    let expected = fixture.capture(None, correction.recorded_at);
    fixture.decision_history.records.judicial.groups.reverse();
    fixture.decision_history.records.administrative.reverse();
    fixture.decision_history.decisions.reverse();
    assert_eq!(fixture.capture(None, correction.recorded_at), expected);
    precautionary_hearing_receipt_with_decision_history_matches(
        &Hasher,
        &expected,
        &fixture.decision_history,
    )
    .unwrap();
}
