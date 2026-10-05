use application::precautionary_measures::MeasureDecisionCommand;
use domain::cases::CaseId;
use uuid::Uuid;

use crate::effect_support::{append_history, empty_history};
use crate::measure_decision_fixtures::Fixture;
use crate::record_support::*;

fn independent_group(serial: u128, measure: u128) -> MeasureDecisionGroupCapture {
    let mut fixture = Fixture::single();
    let values = fixture.command.outcome.changes().unwrap()[0].clone();
    let MeasureEffect::Impose(mut proposal) = values else {
        panic!("imposition expected")
    };
    proposal.id = id(measure);
    fixture.command = MeasureDecisionCommand {
        operation_id: MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + serial)),
        decision_id: MeasureDecisionId::from_uuid(Uuid::from_u128(2000 + serial)),
        outcome: MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(proposal),
        ]))
        .unwrap(),
        ..fixture.command
    };
    fixture.material.result_sources[0].id = id(measure);
    fixture.capture()
}

#[test]
fn empty_selection_resolves_only_an_empty_inventory() {
    let case_id = CaseId::from_uuid(Uuid::from_u128(1));
    assert!(resolve_measure_records(&Hasher, case_id, &[], &empty())
        .unwrap()
        .targets()
        .is_empty());
    let fixture = RecordFixture::initial();
    assert!(resolve_measure_records(&Hasher, case_id, &[], &fixture.history).is_err());
}

#[test]
fn judicial_only_forests_resolve_like_legacy_targets_and_sort_selected_identities() {
    let high = independent_group(1, 90);
    let low = independent_group(2, 70);
    let mut history = empty();
    history
        .judicial
        .groups
        .extend(append_history(&empty_history(), &high).groups);
    history
        .judicial
        .groups
        .extend(append_history(&empty_history(), &low).groups);
    let selections = [reference(&high.measures[0]), reference(&low.measures[0])];
    let checked =
        resolve_measure_records(&Hasher, low.review.case_id, &selections, &history).unwrap();
    assert_eq!(checked.targets().len(), 2);
    for (record, group) in checked.targets().iter().zip([&low, &high]) {
        let prior = &group.measures[0];
        assert_eq!(record.reference(), reference(prior));
        assert_eq!(
            record.record(),
            &OwnedMeasureRecord::Judicial(Box::new(owned(group)))
        );
        assert_eq!(record.last_judicial(), &owned(group));
        assert_eq!(record.context(), &group.review.material.context);
        assert_eq!(record.recorded_at(), prior.recorded_at);
        assert_eq!(record.values(), &prior.result.values);
        assert_eq!(record.sources(), &prior.result.sources);
        assert_eq!(record.projection(), &prior.result.projection);
        assert_eq!(record.support(), &group.decision.support);
    }
}

#[test]
fn mixed_forests_select_exact_records_and_validate_whole_owning_groups() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let independent = independent_group(1, 90);
    let mut history = append_administrative(&first_fixture.history, &first);
    history
        .judicial
        .groups
        .extend(append_history(&empty_history(), &independent).groups);
    history.judicial.groups.reverse();
    let selected = [
        reference(&independent.measures[0]),
        record_reference(&first.records[0]),
    ];
    let checked =
        resolve_measure_records(&Hasher, first_fixture.case_id, &selected, &history).unwrap();
    assert_eq!(checked.targets()[0].reference(), selected[1]);
    assert_eq!(checked.targets()[1].reference(), selected[0]);
    assert!(matches!(
        checked.targets()[0].record(),
        OwnedMeasureRecord::Administrative { .. }
    ));
    assert!(matches!(
        checked.targets()[1].record(),
        OwnedMeasureRecord::Judicial(_)
    ));
}

#[test]
fn external_administrative_inventory_order_does_not_change_exact_resolution() {
    let first_fixture = RecordFixture::initial();
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let third_fixture = RecordFixture::next(&second, &second_fixture.history, 2);
    let third = third_fixture.capture();
    let mut evidence = append_administrative(&third_fixture.history, &third);
    evidence.administrative.reverse();
    let selected = record_reference(&third.records[0]);
    let checked =
        resolve_measure_records(&Hasher, third_fixture.case_id, &[selected], &evidence).unwrap();
    assert_eq!(checked.targets()[0].reference(), selected);
    assert_eq!(checked.targets()[0].values(), &third.review.result.values);
}

#[test]
fn original_bare_measure_remains_selectable_with_its_own_exact_historical_closure() {
    let first_fixture = RecordFixture::initial();
    let judicial = owned(&first_fixture.history.judicial.groups[0].capture);
    let first = first_fixture.capture();
    let second_fixture = RecordFixture::next(&first, &first_fixture.history, 1);
    let second = second_fixture.capture();
    let current_history = append_administrative(&second_fixture.history, &second);
    let current = resolve_measure_records(
        &Hasher,
        first_fixture.case_id,
        &[record_reference(&second.records[0])],
        &current_history,
    )
    .unwrap();
    let historical = resolve_measure_records(
        &Hasher,
        first_fixture.case_id,
        &[reference(&judicial.capture)],
        &first_fixture.history,
    )
    .unwrap();
    assert_eq!(
        historical.targets()[0].record(),
        &OwnedMeasureRecord::Judicial(Box::new(judicial.clone()))
    );
    assert_eq!(
        historical.targets()[0].values(),
        &judicial.capture.result.values
    );
    assert_eq!(
        historical.targets()[0].recorded_at(),
        judicial.capture.recorded_at
    );
    assert_ne!(
        historical.targets()[0].values(),
        current.targets()[0].values()
    );
    assert_eq!(current.targets()[0].last_judicial(), &judicial);
}
