use crate::measure_decision_fixtures::Fixture;
use crate::record_decision_support::*;
use crate::record_review_support::RecordReviewFixture;
use time::Duration;

#[test]
fn a_review_of_a_corrected_record_can_anchor_a_v2_decision_with_no_measure_change() {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let records = append_administrative(&correction.history, &administrative);
    let mut hearing_fixture = RecordReviewFixture::schedule(
        vec![record_reference(&administrative.records[0])],
        records.clone(),
    );
    hearing_fixture.hearing.sources.participants.clear();
    if let application::precautionary_hearings::PrecautionaryHearingChange::Schedule {
        values,
        ..
    } = &mut hearing_fixture.hearing.command.change
    {
        let mut input = crate::record_review_support::values_input(values);
        input.participants.clear();
        *values = domain::precautionary_hearings::PrecautionaryHearingValues::new(input).unwrap();
    }
    let hearing = hearing_fixture.capture(None, administrative.recorded_at + Duration::seconds(1));
    let mut fixture = FixtureV2::initial(Fixture::no_change());
    fixture.identities(10);
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.clone(),
    )));
    fixture.history.records = records;
    fixture.recorded_at = hearing.recorded_at + Duration::seconds(1);
    let group = fixture.capture();
    assert!(group.measures.is_empty());
    assert_eq!(group.decision.anchor, fixture.material.anchor);
    measure_decision_group_v2_matches(&Hasher, &group, &fixture.history).unwrap();
    measure_group_origin_v2(&Hasher, &group, &fixture.history).unwrap();
}
