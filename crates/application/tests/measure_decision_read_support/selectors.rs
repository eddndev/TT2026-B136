use super::*;

#[test]
fn listing_keeps_original_decisions_after_later_measure_revisions_exist() {
    let initial = operation(10);
    let later = confirmed(&initial, 20);
    let expected = page(
        initial.group.review.case_id,
        vec![initial.clone(), later.clone()],
    );
    let returned = expected.clone();
    let actor = reader(Role::Paralegal);
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let result = service(store, identity(&actor))
        .list(
            "session",
            initial.group.review.case_id,
            MeasureDecisionReadQuery::default(),
        )
        .unwrap();
    assert_eq!(result, expected);
    assert_eq!(result.items[0].group.measures[0].result.revision.get(), 1);
    assert_eq!(result.items[1].group.measures[0].result.revision.get(), 2);
    assert_eq!(
        result.items[1].measure_history.groups[0].capture,
        initial.group
    );
}

#[test]
fn decision_and_operation_selectors_never_substitute_another_valid_later_group() {
    let original = operation(10);
    let later = confirmed(&original, 20);
    let actor = reader(Role::Litigator);
    for kind in [ReadKind::Get, ReadKind::Operation] {
        let service = service(
            successful_store(&actor, &original, later.clone(), kind),
            identity(&actor),
        );
        assert!(read(&service, kind, &original).is_err());
    }
}

#[test]
fn exact_operation_read_returns_its_full_ancestor_closure() {
    let initial = operation(10);
    let original = confirmed(&initial, 20);
    let actor = reader(Role::Paralegal);
    let service = service(
        successful_store(&actor, &original, original.clone(), ReadKind::Operation),
        identity(&actor),
    );
    assert_eq!(
        read(&service, ReadKind::Operation, &original).unwrap(),
        vec![original]
    );
}

#[test]
fn an_original_no_change_decision_is_readable_without_measure_rows() {
    let mut fixture = root_fixture(10);
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(
            domain::hearings::HearingNote::new("No change declared in the decision").unwrap(),
        ))
        .unwrap();
    fixture.material.result_sources.clear();
    let original = stored(fixture.capture(), empty_history());
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let service = service(
            successful_store(&actor, &original, original.clone(), kind),
            identity(&actor),
        );
        let result = read(&service, kind, &original).unwrap();
        assert_eq!(result, vec![original.clone()]);
        assert!(result[0].group.measures.is_empty());
    }
}
