use super::*;

#[test]
fn review_anchor_and_affected_measure_resolve_their_different_complete_owners() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let mut independent = fresh(&seed.command);
    independent.outcome = impositions(&seed.subject, 1);
    let other = persist(&db, seed.actor.clone(), independent);
    let anchor_target = reference(&first.group.measures[0]);
    let affected = reference(&other.group.measures[0]);
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        hearing_command(&db, &seed, vec![anchor_target]),
    );
    let stored = persist(
        &db,
        seed.actor.clone(),
        effects(
            &seed,
            &hearing,
            vec![MeasureEffect::Confirm { previous: affected }],
        ),
    );

    assert_ne!(anchor_target.id(), affected.id());
    assert_eq!(stored.group.measures.len(), 1);
    assert_eq!(stored.group.measures[0].result.previous, Some(affected));
    assert_eq!(stored.group.review.material.predecessors.len(), 1);
    assert_eq!(
        stored.group.review.material.predecessors[0].capture,
        other.group.measures[0]
    );
    assert_groups(&stored.measure_history, &[&first, &other]);
    assert_anchor(&stored, &hearing);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn no_measure_change_retains_its_review_anchor_and_real_target_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        hearing_command(&db, &seed, vec![reference(&first.group.measures[0])]),
    );
    let mut command = no_change(&seed.command);
    command.anchor = Some(anchor(&hearing));
    let stored = persist(&db, seed.actor.clone(), command);

    assert!(stored.group.measures.is_empty());
    assert!(stored.group.review.results.is_empty());
    assert!(stored.group.review.material.predecessors.is_empty());
    assert_groups(&stored.measure_history, &[&first]);
    assert_anchor(&stored, &hearing);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_measure_decisions WHERE operation_id=$1",
            &[&stored.origin.operation_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn review_of_an_anchored_decision_preserves_the_alternating_durable_history() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let old = reference(&first.group.measures[0]);
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        hearing_command(&db, &seed, vec![old]),
    );
    let next = persist(
        &db,
        seed.actor.clone(),
        effects(
            &seed,
            &hearing,
            vec![MeasureEffect::Confirm { previous: old }],
        ),
    );
    let selected = reference(&next.group.measures[0]);
    let later = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        hearing_command(&db, &seed, vec![selected]),
    );

    assert_anchor(&next, &hearing);
    assert_groups(&next.measure_history, &[&first]);
    assert_groups(&later.history.measure_history, &[&first, &next]);
    assert_eq!(
        later.capture.review.resolved_values.review_targets(),
        &[selected]
    );
    assert_eq!(later.history.captures, vec![later.capture.clone()]);
    reopened(&db, &seed.actor, &next);
    reopened_hearing(&db, &seed.actor, &later);
}

#[test]
fn an_old_anchor_target_and_newer_effect_target_of_one_measure_keep_distinct_revisions() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let old = reference(&first.group.measures[0]);
    let mut change = fresh(&seed.command);
    change.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Confirm { previous: old },
    ]))
    .unwrap();
    let second = persist(&db, seed.actor.clone(), change);
    let newer = reference(&second.group.measures[0]);
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        hearing_command(&db, &seed, vec![old]),
    );
    let third = persist(
        &db,
        seed.actor.clone(),
        effects(
            &seed,
            &hearing,
            vec![MeasureEffect::Confirm { previous: newer }],
        ),
    );

    assert_eq!(old.id(), newer.id());
    assert_eq!(old.revision().get(), 1);
    assert_eq!(newer.revision().get(), 2);
    assert_eq!(third.group.measures[0].result.revision.get(), 3);
    assert_eq!(third.group.measures[0].result.previous, Some(newer));
    assert_eq!(
        hearing.capture.review.resolved_values.review_targets(),
        &[old]
    );
    assert_groups(&third.measure_history, &[&first, &second]);
    assert_anchor(&third, &hearing);
    reopened(&db, &seed.actor, &third);
}
