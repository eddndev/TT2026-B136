use super::*;

#[test]
fn older_review_prefix_is_proved_without_entering_the_selected_imposition_anchor_wire_closure() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let hearing_one = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        hearing_command(&db, &seed, vec![reference(&first.group.measures[0])]),
    );
    let hearing_two = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        replace_targets(&hearing_one, vec![]),
    );
    let mut command = fresh(&seed.command);
    command.outcome = impositions(&seed.subject, 1);
    command.anchor = Some(anchor(&hearing_two));
    let stored = persist(&db, seed.actor.clone(), command);

    assert_eq!(
        hearing_two.history.captures,
        vec![hearing_one.capture.clone(), hearing_two.capture.clone()]
    );
    assert_groups(&hearing_two.history.measure_history, &[&first]);
    assert!(stored.measure_history.groups.is_empty());
    assert!(stored.group.review.material.predecessors.is_empty());
    assert_anchor(&stored, &hearing_two);
    reopened(&db, &seed.actor, &stored);
    reopened_hearing(&db, &seed.actor, &hearing_two);
}

#[test]
fn old_and_cancelled_anchor_revisions_keep_original_receipts_after_later_hearing_changes() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.seed.actor.clone(), seed.command);
    let second = crate::hearing_fixture::persist(
        &db,
        seed.seed.actor.clone(),
        replace_targets(&seed.hearing, vec![]),
    );
    let cancelled = crate::hearing_fixture::persist(
        &db,
        seed.seed.actor.clone(),
        crate::hearing_fixture::cancellation(&second),
    );
    let mut old_command = no_change(&seed.seed.command);
    old_command.anchor = Some(anchor(&seed.hearing));
    let old = persist(&db, seed.seed.actor.clone(), old_command);
    let mut cancelled_command = no_change(&seed.seed.command);
    cancelled_command.anchor = Some(anchor(&cancelled));
    let selected_cancel = persist(&db, seed.seed.actor.clone(), cancelled_command);

    assert_eq!(cancelled.capture.review.status, HearingStatus::Cancelled);
    assert_anchor(&original, &seed.hearing);
    assert_anchor(&old, &seed.hearing);
    assert_anchor(&selected_cancel, &cancelled);
    assert!(old.measure_history.groups.is_empty());
    assert!(selected_cancel.measure_history.groups.is_empty());
    for operation in [&original, &old, &selected_cancel] {
        reopened(&db, &seed.seed.actor, operation);
    }
}

#[test]
fn shared_hearing_revisions_do_not_create_a_cycle_through_a_later_review() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.seed.actor.clone(), seed.command);
    let target = reference(&first.group.measures[0]);
    let second_hearing = crate::hearing_fixture::persist(
        &db,
        seed.seed.actor.clone(),
        replace_targets(&seed.hearing, vec![target]),
    );
    let mut command = no_change(&seed.seed.command);
    command.anchor = Some(anchor(&second_hearing));
    let second_group = persist(&db, seed.seed.actor.clone(), command);

    assert_anchor(&first, &seed.hearing);
    assert_anchor(&second_group, &second_hearing);
    assert_groups(&second_group.measure_history, &[&first]);
    assert_eq!(
        second_hearing.history.captures,
        vec![seed.hearing.capture.clone(), second_hearing.capture.clone()]
    );
    assert_groups(&second_hearing.history.measure_history, &[&first]);
    reopened(&db, &seed.seed.actor, &second_group);
    reopened_hearing(&db, &seed.seed.actor, &second_hearing);
}
