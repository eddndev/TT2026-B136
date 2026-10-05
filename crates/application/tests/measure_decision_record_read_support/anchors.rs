use super::*;
use crate::record_review_support::RecordReviewFixture;
use application::precautionary_hearings::PrecautionaryHearingChange;

fn anchored(serial: u128, venue: &str, cancelled: bool) -> MeasureDecisionRecordReceipt {
    let correction = RecordFixture::initial();
    let administrative = correction.capture();
    let records = append_administrative(&correction.history, &administrative);
    let mut fixture = RecordReviewFixture::schedule(
        vec![record_reference(&administrative.records[0])],
        records.clone(),
    );
    fixture.hearing.sources.participants.clear();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.hearing.command.change
    else {
        unreachable!()
    };
    let mut input = crate::record_review_support::values_input(values);
    input.participants.clear();
    input.venue = domain::hearings::HearingVenue::new(venue).unwrap();
    *values = domain::precautionary_hearings::PrecautionaryHearingValues::new(input).unwrap();
    let first = fixture.capture(None, administrative.recorded_at + Duration::seconds(1));
    let hearing = if cancelled {
        RecordReviewFixture::cancel(&first, records.clone())
            .capture(Some(&first), first.recorded_at + Duration::seconds(1))
    } else {
        first
    };
    let mut fixture = FixtureV2::initial(crate::measure_decision_fixtures::Fixture::no_change());
    fixture.identities(serial);
    fixture.material.context = hearing.review.observed_context.clone();
    fixture.command.context = expectation(&fixture.material.context);
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
    from_v2(&fixture)
}

#[test]
fn review_and_cancelled_review_anchors_keep_original_c_target_wire_closure() {
    for cancelled in [false, true] {
        let original = anchored(10, "Declared room", cancelled);
        let MeasureDecisionRecordReceipt::V2(value) = &original else {
            unreachable!()
        };
        assert!(value.group.measures.is_empty());
        assert_eq!(value.record_history.records.judicial.groups.len(), 1);
        assert_eq!(value.record_history.records.administrative.len(), 1);
        let actor = reader(Role::Paralegal);
        for kind in READS {
            assert_eq!(
                read(
                    &service(
                        successful_store(&actor, &original, original.clone(), kind),
                        identity(&actor)
                    ),
                    kind,
                    &original
                )
                .unwrap(),
                vec![original.clone()]
            );
        }
        let mut missing = original.clone();
        let MeasureDecisionRecordReceipt::V2(value) = &mut missing else {
            unreachable!()
        };
        value.record_history.records.administrative.clear();
        reject(&original, &missing);
    }
}

#[test]
fn mixed_read_pages_reject_contradictory_full_anchors_at_one_hearing_revision() {
    let first = anchored(10, "First room", false);
    let second = anchored(20, "Different room", false);
    assert!(list_result(
        MeasureDecisionReadQuery::default(),
        page(first.case_id(), vec![first, second])
    )
    .is_err());
}

#[test]
fn mixed_receipt_validation_rejects_forged_anchor_even_with_original_group_present() {
    let expected = anchored(10, "Declared room", false);
    let mut returned = expected.clone();
    let MeasureDecisionRecordReceipt::V2(value) = &mut returned else {
        unreachable!()
    };
    let Some(MeasureDecisionAnchorMaterial::Precautionary(hearing)) =
        &mut value.group.review.material.anchor
    else {
        unreachable!()
    };
    hearing.review.actor.email = "invented-anchor@example.test".into();
    reject(&expected, &returned);
}
