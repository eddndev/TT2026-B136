use crate::measure_dependency_support::*;
use time::Duration;

#[test]
fn a_zero_row_legacy_decision_anchored_to_review_is_a_known_dependant() {
    let base = Fixture::single().capture();
    let selected = reference(&base.measures[0]);
    let mut inventory = judicial_inventory(&base);
    let initial_history = inventory.records.clone();
    let hearing = DecisionReviewFixture::schedule(vec![selected], initial_history.clone())
        .capture(None, base.recorded_at);
    let group = anchor_v1(&hearing, &initial_history.records.judicial, 1);
    assert!(group.measures.is_empty());
    inventory.records.records.judicial =
        crate::effect_support::append_history(&initial_history.records.judicial, &group);
    inventory
        .hearings
        .push(hearing_prefix(vec![hearing.clone()], &initial_history));

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[
            MeasureAdministrativeDependant::Review {
                hearing: hearing_ref(&hearing),
                target: selected,
            },
            MeasureAdministrativeDependant::ReviewAnchor {
                owner: owner_v1(&group),
                hearing: hearing_ref(&hearing),
                target: selected,
            },
        ]
    );
}

#[test]
fn a_zero_row_versioned_decision_reports_its_review_of_a_genuine_m2() {
    let fixture = judicial_fixture();
    let base = fixture.capture();
    let selected = reference_v2(&base.measures[0]);
    let history = append_v2(&fixture.history, &base);
    let hearing = DecisionReviewFixture::schedule(vec![selected], history.clone())
        .capture(None, base.recorded_at);
    let group = anchor_v2(&hearing, &history, 20);
    let inventory = MeasureAdministrativeDependencyInventory {
        records: append_v2(&history, &group),
        hearings: vec![hearing_prefix(vec![hearing.clone()], &history)],
    };
    assert!(group.measures.is_empty());

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[
            MeasureAdministrativeDependant::Review {
                hearing: hearing_ref(&hearing),
                target: selected,
            },
            MeasureAdministrativeDependant::ReviewAnchor {
                owner: owner_v2(&group),
                hearing: hearing_ref(&hearing),
                target: selected,
            },
        ]
    );
}

#[test]
fn every_replaced_and_cancelled_revision_retains_its_exact_review_use() {
    let base = Fixture::single().capture();
    let initial = judicial_inventory(&base).records;
    let first_target = reference(&base.measures[0]);
    let first = DecisionReviewFixture::schedule(vec![first_target], initial.clone())
        .capture(None, base.recorded_at);
    let later_fixture = crate::effect_support::LaterFixture::confirm(&base);
    let group = later_fixture.clone().capture();
    let mut records = initial.clone();
    records.records.judicial =
        crate::effect_support::append_history(&later_fixture.evidence, &group);
    let second_target = reference(&group.measures[0]);
    let second = DecisionReviewFixture::replace(&first, vec![second_target], records.clone())
        .capture(Some(&first), group.recorded_at);
    let third = DecisionReviewFixture::cancel(&second, records.clone())
        .capture(Some(&second), group.recorded_at + Duration::seconds(1));
    let inventory = MeasureAdministrativeDependencyInventory {
        records,
        hearings: vec![hearing_prefix(
            vec![first.clone(), second.clone(), third.clone()],
            &initial,
        )],
    };

    assert_eq!(
        inspect(first_target, &inventory).dependants(),
        &[
            MeasureAdministrativeDependant::Judicial {
                owner: owner_v1(&group),
                target: first_target,
            },
            MeasureAdministrativeDependant::Review {
                hearing: hearing_ref(&first),
                target: first_target,
            },
        ]
    );
    assert_eq!(
        inspect(second_target, &inventory).dependants(),
        &[
            MeasureAdministrativeDependant::Review {
                hearing: hearing_ref(&second),
                target: second_target,
            },
            MeasureAdministrativeDependant::Review {
                hearing: hearing_ref(&third),
                target: second_target,
            },
        ]
    );
}

#[test]
fn a_review_of_c_after_m2_reports_the_effective_administrative_reference() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let fixture = AdministrativeFixture::after(&group, &judicial.history, 10);
    let correction = fixture.capture();
    let records = append_administrative_decision_history(&fixture.history, &correction);
    let selected = record_reference(&correction.records[0]);
    let hearing = DecisionReviewFixture::schedule(vec![selected], records.clone())
        .capture(None, correction.recorded_at);
    let inventory = MeasureAdministrativeDependencyInventory {
        records: records.clone(),
        hearings: vec![hearing_prefix(vec![hearing.clone()], &records)],
    };

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[MeasureAdministrativeDependant::Review {
            hearing: hearing_ref(&hearing),
            target: selected,
        },]
    );
    assert_eq!(
        inspect(reference_v2(&group.measures[0]), &inventory).dependants(),
        &[MeasureAdministrativeDependant::Administrative {
            owner: administrative_owner(&correction),
            target: reference_v2(&group.measures[0]),
        },]
    );
}
