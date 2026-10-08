use crate::effect_support::empty_history;
use crate::measure_decision_fixtures::Fixture;
use crate::record_support::*;

#[test]
fn thirty_two_mixed_selections_are_normalized_without_dropping_unselected_owner_rows() {
    let mut initial = Fixture::single();
    let sources = initial.material.result_sources[0].sources.clone();
    let values = MeasureValues::new(crate::measure_source_support::input(&sources));
    for value in 71..=101 {
        initial.add_imposition(value, values.clone(), sources.clone());
    }
    let group = initial.capture();
    assert_eq!(group.measures.len(), 32);
    let fixture = RecordFixture::from_first(CorrectionFixture::from_group(
        &group,
        &empty_history(),
        id(70),
    ));
    let capture = fixture.capture();
    let evidence = append_administrative(&fixture.history, &capture);
    let mut selected: Vec<_> = group.measures.iter().skip(1).map(reference).collect();
    selected.push(record_reference(&capture.records[0]));
    selected.reverse();
    let checked = resolve_measure_records(&Hasher, fixture.case_id, &selected, &evidence).unwrap();
    assert_eq!(checked.targets().len(), 32);
    for (record, expected_id) in checked.targets().iter().zip(70..=101) {
        assert_eq!(record.reference().id(), id(expected_id));
        assert_eq!(
            record.reference().revision().get(),
            if expected_id == 70 { 2 } else { 1 }
        );
    }
    let only_corrected = resolve_measure_records(
        &Hasher,
        fixture.case_id,
        &[record_reference(&capture.records[0])],
        &evidence,
    )
    .unwrap();
    assert_eq!(only_corrected.targets().len(), 1);
    assert_eq!(evidence.judicial.groups[0].capture.measures.len(), 32);
}

#[test]
fn mixed_chain_accepts_256_total_owners_and_rejects_the_next_correction_candidate() {
    let mut initial = Fixture::single();
    let source = crate::measure_source_support::Fixture::unknown(true);
    initial.replace_values(id(70), source.values);
    initial.material.result_sources[0].sources = source.sources;
    let group = initial.capture();
    let mut fixture = RecordFixture::from_first(CorrectionFixture::from_group(
        &group,
        &empty_history(),
        id(70),
    ));
    let mut capture = fixture.capture();
    for serial in 1..255 {
        fixture = RecordFixture::next(&capture, &fixture.history, serial);
        capture = fixture.capture();
    }
    let evidence = append_administrative(&fixture.history, &capture);
    assert_eq!(evidence.judicial.groups.len(), 1);
    assert_eq!(evidence.administrative.len(), 255);
    let selected = record_reference(&capture.records[0]);
    let checked =
        resolve_measure_records(&Hasher, fixture.case_id, &[selected], &evidence).unwrap();
    assert_eq!(checked.targets()[0].reference().revision().get(), 256);
    assert_eq!(
        checked.targets()[0].last_judicial(),
        &OwnedJudicialMeasure::V1(Box::new(owned(&group)))
    );
    let over_budget = RecordFixture::next(&capture, &fixture.history, 255);
    assert_eq!(over_budget.history.administrative.len(), 255);
    assert!(over_budget.prepare().is_err());
}
