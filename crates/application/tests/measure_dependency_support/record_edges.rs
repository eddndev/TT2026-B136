use crate::measure_dependency_support::*;

#[test]
fn an_administrative_command_reports_only_its_exact_selected_predecessor() {
    let fixture = RecordFixture::initial();
    let capture = fixture.capture();
    let selected = fixture.command.target;
    let inventory = MeasureAdministrativeDependencyInventory {
        records: MeasureDecisionRecordHistoryEvidence {
            records: append_administrative(&fixture.history, &capture),
            decisions: vec![],
        },
        hearings: vec![],
    };

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[MeasureAdministrativeDependant::Administrative {
            owner: administrative_owner(&capture),
            target: selected,
        },]
    );
    assert!(inspect(record_reference(&capture.records[0]), &inventory)
        .dependants()
        .is_empty());
}

#[test]
fn a_legacy_effect_reports_its_actual_owning_group() {
    let base = Fixture::single().capture();
    let later_fixture = crate::effect_support::LaterFixture::confirm(&base);
    let group = later_fixture.clone().capture();
    let mut inventory = judicial_inventory(&base);
    inventory.records.records.judicial =
        crate::effect_support::append_history(&later_fixture.evidence, &group);
    let selected = reference(&base.measures[0]);

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[MeasureAdministrativeDependant::Judicial {
            owner: owner_v1(&group),
            target: selected,
        },]
    );
    assert!(inspect(reference(&group.measures[0]), &inventory)
        .dependants()
        .is_empty());
}

#[test]
fn a_versioned_effect_selects_the_correction_not_its_retained_judicial_reference() {
    let first = RecordFixture::initial();
    let correction = first.capture();
    let fixture = FixtureV2::confirm(&correction, &first.history);
    let group = fixture.capture();
    let inventory = MeasureAdministrativeDependencyInventory {
        records: append_v2(&fixture.history, &group),
        hearings: vec![],
    };
    let selected = record_reference(&correction.records[0]);

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[MeasureAdministrativeDependant::Judicial {
            owner: owner_v2(&group),
            target: selected,
        },]
    );
    assert_eq!(
        inspect(first.command.target, &inventory).dependants(),
        &[MeasureAdministrativeDependant::Administrative {
            owner: administrative_owner(&correction),
            target: first.command.target,
        },]
    );
}

#[test]
fn correction_after_m2_reports_the_genuine_m2_reference() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let fixture = AdministrativeFixture::after(&group, &judicial.history, 10);
    let correction = fixture.capture();
    let inventory = MeasureAdministrativeDependencyInventory {
        records: append_administrative_decision_history(&fixture.history, &correction),
        hearings: vec![],
    };
    let selected = reference_v2(&group.measures[0]);

    assert_eq!(
        inspect(selected, &inventory).dependants(),
        &[MeasureAdministrativeDependant::Administrative {
            owner: administrative_owner(&correction),
            target: selected,
        },]
    );
    assert!(
        inspect(record_reference(&correction.records[0]), &inventory)
            .dependants()
            .is_empty()
    );
}

#[test]
fn inspection_accepts_an_entered_in_error_target_without_implying_eligibility() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let mut fixture = AdministrativeFixture::after(&group, &judicial.history, 10);
    fixture.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = fixture.capture();
    let inventory = MeasureAdministrativeDependencyInventory {
        records: append_administrative_decision_history(&fixture.history, &marked),
        hearings: vec![],
    };
    let selected = record_reference(&marked.records[0]);

    assert_eq!(
        marked.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    assert_eq!(inspect(selected, &inventory).target(), selected);
    assert!(inspect(selected, &inventory).dependants().is_empty());
}

#[test]
fn using_a_sibling_does_not_create_a_dependency_on_the_selected_row() {
    let base = Fixture::multiple().capture();
    let later_fixture = crate::effect_support::LaterFixture::confirm(&base);
    let group = later_fixture.clone().capture();
    let mut inventory = judicial_inventory(&base);
    inventory.records.records.judicial =
        crate::effect_support::append_history(&later_fixture.evidence, &group);

    assert!(inspect(reference(&base.measures[1]), &inventory)
        .dependants()
        .is_empty());
    assert_eq!(
        inspect(reference(&base.measures[0]), &inventory)
            .dependants()
            .len(),
        1
    );
}
