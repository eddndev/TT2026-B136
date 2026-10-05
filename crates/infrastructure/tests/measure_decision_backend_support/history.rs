use super::*;
use domain::precautionary_hearings::PrecautionaryMeasureRef;

mod effects;
mod integrity;
mod substitution;

fn reference(capture: &MeasureCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

fn command(base: &MeasureDecisionCommand, effects: Vec<MeasureEffect>) -> MeasureDecisionCommand {
    let mut next = fresh(base);
    next.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    next
}

fn member(group: &MeasureDecisionStoredOperation, id: MeasureId) -> &MeasureCapture {
    group
        .group
        .measures
        .iter()
        .find(|row| row.result.id == id)
        .unwrap()
}

fn ancestors(
    actual: &MeasureDecisionStoredOperation,
    expected: &[&MeasureDecisionStoredOperation],
) {
    assert_eq!(actual.measure_history.groups.len(), expected.len());
    for original in expected {
        let selected = actual
            .measure_history
            .groups
            .iter()
            .find(|entry| entry.origin.operation_id == original.origin.operation_id)
            .unwrap();
        assert_eq!(selected.origin, original.origin);
        assert_eq!(selected.capture, original.group);
    }
    assert!(actual
        .measure_history
        .groups
        .iter()
        .all(|entry| { entry.origin.operation_id != actual.origin.operation_id }));
}

fn same_operation(
    actual: &MeasureDecisionStoredOperation,
    expected: &MeasureDecisionStoredOperation,
) {
    assert_eq!(actual.group, expected.group);
    assert_eq!(actual.origin, expected.origin);
    let mut actual_ancestors = actual.measure_history.groups.clone();
    let mut expected_ancestors = expected.measure_history.groups.clone();
    actual_ancestors.sort_by_key(|entry| entry.origin.operation_id.as_uuid());
    expected_ancestors.sort_by_key(|entry| entry.origin.operation_id.as_uuid());
    assert_eq!(actual_ancestors, expected_ancestors);
}

fn reopened(db: &Fixture, actor: &Principal, expected: &MeasureDecisionStoredOperation) {
    let query = reads(db, actor.clone());
    same_operation(
        &query
            .get("session", db.case, expected.origin.decision_id)
            .unwrap(),
        expected,
    );
    same_operation(
        &query
            .get_operation("session", db.case, expected.origin.operation_id)
            .unwrap(),
        expected,
    );
    let replay = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("historical decision replay must skip documentary admission")
        }))),
    );
    same_operation(
        &replay
            .submit(
                "session",
                db.case,
                expected.group.review.command.clone(),
                confirmation(&expected.group.review),
            )
            .unwrap(),
        expected,
    );
}

#[test]
fn confirmation_reopens_with_exact_ancestry_and_preserves_original_group_bytes() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let original_bytes = measure_decision_group_bytes(&first.group).unwrap();
    let next = command(
        &seed.command,
        first
            .group
            .measures
            .iter()
            .map(|row| MeasureEffect::Confirm {
                previous: reference(row),
            })
            .collect(),
    );
    let confirmed = persist(&db, seed.actor.clone(), next);

    assert_eq!(confirmed.group.measures.len(), 2);
    ancestors(&confirmed, &[&first]);
    for previous in &first.group.measures {
        let result = &member(&confirmed, previous.result.id).result;
        assert_eq!(result.revision.get(), 2);
        assert_eq!(result.action, MeasureCaptureAction::Confirm);
        assert_eq!(result.previous, Some(reference(previous)));
        assert_eq!(result.origin, previous.result.origin);
        assert_eq!(result.values, previous.result.values);
        assert_eq!(result.sources, previous.result.sources);
        assert_eq!(result.projection, previous.result.projection);
    }
    reopened(&db, &seed.actor, &first);
    reopened(&db, &seed.actor, &confirmed);
    let original = reads(&db, seed.actor)
        .get("session", db.case, first.origin.decision_id)
        .unwrap();
    assert_eq!(
        measure_decision_group_bytes(&original.group).unwrap(),
        original_bytes
    );
}

#[test]
fn selecting_one_member_retains_its_siblings_complete_independent_ancestry() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let mut independent = fresh(&seed.command);
    independent.outcome = impositions(&seed.subject, 2);
    let second = persist(&db, seed.actor.clone(), independent);
    let selected = first.group.measures[0].result.id;
    let joined = persist(
        &db,
        seed.actor.clone(),
        command(
            &seed.command,
            vec![
                MeasureEffect::Confirm {
                    previous: reference(&first.group.measures[0]),
                },
                MeasureEffect::Confirm {
                    previous: reference(&second.group.measures[0]),
                },
            ],
        ),
    );
    ancestors(&joined, &[&first, &second]);
    let last = persist(
        &db,
        seed.actor.clone(),
        command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(member(&joined, selected)),
            }],
        ),
    );

    assert_eq!(last.group.measures.len(), 1);
    assert_eq!(last.group.measures[0].result.revision.get(), 3);
    assert_eq!(
        last.group.measures[0].result.origin,
        first.group.measures[0].result.origin
    );
    ancestors(&last, &[&first, &second, &joined]);
    assert_eq!(
        last.measure_history
            .groups
            .iter()
            .map(|entry| entry.capture.measures.len())
            .sum::<usize>(),
        6
    );
    reopened(&db, &seed.actor, &last);
}
