use crate::effect_support::{empty_history, substitution, LaterFixture};
use crate::measure_decision_fixtures::Fixture;
use crate::record_review_support::*;

#[test]
fn terminal_judicial_actions_remain_selectable_through_valid_corrected_records() {
    let initial = Fixture::single().capture();
    for kind in 0..3 {
        let mut later = if kind == 2 {
            substitution(&initial, &[90])
        } else {
            LaterFixture::confirm(&initial)
        };
        let previous = reference(&initial.measures[0]);
        if kind != 2 {
            later.effects(vec![if kind == 0 {
                MeasureEffect::Revoke { previous }
            } else {
                MeasureEffect::Cease { previous }
            }]);
        }
        let ancestors = later.evidence.clone();
        let group = later.capture();
        let correction =
            RecordFixture::from_first(CorrectionFixture::from_group(&group, &ancestors, id(70)));
        let record = correction.capture();
        let evidence = append_administrative(&correction.history, &record);
        let fixture = RecordReviewFixture::schedule(
            vec![record_reference(&record.records[0])],
            evidence.clone(),
        );
        let hearing = fixture.capture(None, record.recorded_at);
        assert_eq!(
            hearing.review.resolved_values.review_targets(),
            &[record_reference(&record.records[0])]
        );
        precautionary_hearing_receipt_with_record_history_matches(&Hasher, &hearing, &evidence)
            .unwrap();
    }
}

#[test]
fn terminal_bare_judicial_records_keep_their_existing_review_eligibility() {
    let initial = Fixture::single().capture();
    let mut later = LaterFixture::confirm(&initial);
    later.effects(vec![MeasureEffect::Revoke {
        previous: reference(&initial.measures[0]),
    }]);
    let ancestors = later.evidence.clone();
    let group = later.capture();
    let evidence = MeasureRecordHistoryEvidence {
        judicial: crate::effect_support::append_history(&ancestors, &group),
        administrative: vec![],
    };
    let fixture =
        RecordReviewFixture::schedule(vec![reference(&group.measures[0])], evidence.clone());
    let hearing = fixture.capture(None, group.recorded_at);
    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &hearing, &evidence)
        .unwrap();
}

#[test]
fn one_review_can_select_judicial_and_administrative_siblings_in_uuid_order() {
    let group = Fixture::multiple().capture();
    let correction = RecordFixture::from_first(CorrectionFixture::from_group(
        &group,
        &empty_history(),
        id(80),
    ));
    let record = correction.capture();
    let evidence = append_administrative(&correction.history, &record);
    let fixture = RecordReviewFixture::schedule(
        vec![
            record_reference(&record.records[0]),
            reference(&group.measures[0]),
        ],
        evidence.clone(),
    );
    let hearing = fixture.capture(None, record.recorded_at);
    assert_eq!(
        hearing.review.resolved_values.review_targets(),
        &[
            reference(&group.measures[0]),
            record_reference(&record.records[0]),
        ]
    );
    assert_eq!(evidence.judicial.groups[0].capture, group);
    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &hearing, &evidence)
        .unwrap();
}

#[test]
fn review_accepts_a_reordered_administrative_ancestry_without_substituting_earlier_values() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let mut history = append_administrative(&second_fixture.history, &second);
    history.administrative.reverse();
    let selected = record_reference(&second.records[0]);
    let fixture = RecordReviewFixture::schedule(vec![selected], history.clone());
    let hearing = fixture.capture(None, second.recorded_at);
    assert_eq!(hearing.review.resolved_values.review_targets(), &[selected]);
    precautionary_hearing_receipt_with_record_history_matches(&Hasher, &hearing, &history).unwrap();
}

#[test]
fn two_full_review_target_lists_preserve_sixty_four_exact_refs_in_their_history_union() {
    let mut initial = Fixture::single();
    let sources = initial.material.result_sources[0].sources.clone();
    let values = MeasureValues::new(crate::measure_source_support::input(&sources));
    for value in 71..=101 {
        initial.add_imposition(value, values.clone(), sources.clone());
    }
    let group = initial.capture();
    let judicial = crate::effect_support::append_history(&empty_history(), &group);
    let mut evidence = MeasureRecordHistoryEvidence {
        judicial,
        administrative: vec![],
    };
    let old_targets = group.measures.iter().map(reference).collect();
    let first = RecordReviewFixture::schedule(old_targets, evidence.clone())
        .capture(None, group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_record_history(&Hasher, &first, &evidence).unwrap();
    let mut new_targets = Vec::new();
    for (index, row) in group.measures.iter().enumerate() {
        let mut fixture = RecordFixture::from_first(CorrectionFixture::from_group(
            &group,
            &empty_history(),
            row.result.id,
        ));
        fixture.command.operation_id =
            MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(500 + index as u128));
        let correction = fixture.capture();
        new_targets.push(record_reference(&correction.records[0]));
        let origin =
            measure_administrative_origin_with_history(&Hasher, &correction, &fixture.history)
                .unwrap();
        evidence.administrative.push(MeasureAdministrativeEvidence {
            origin,
            capture: correction,
        });
    }
    new_targets.reverse();
    let second = RecordReviewFixture::replace(&first, new_targets, evidence.clone())
        .capture(Some(&first), group.recorded_at + time::Duration::seconds(2));
    assert_eq!(first.review.resolved_values.review_targets().len(), 32);
    assert_eq!(second.review.resolved_values.review_targets().len(), 32);
    for (previous, next) in first
        .review
        .resolved_values
        .review_targets()
        .iter()
        .zip(second.review.resolved_values.review_targets())
    {
        assert_eq!(previous.id(), next.id());
        assert_eq!(previous.revision().get(), 1);
        assert_eq!(next.revision().get(), 2);
    }
    precautionary_hearing_transition_with_record_history_matches(
        &Hasher, &first, &second, &evidence,
    )
    .unwrap();
    precautionary_hearing_history_with_record_history_matches(
        &Hasher,
        &[first, second],
        &origin,
        &evidence,
    )
    .unwrap();
}
