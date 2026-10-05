use super::*;

#[test]
fn review_schedule_returns_actual_selected_groups_and_exact_original_hearing() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let review = workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let stored = workflow
        .submit(
            "session",
            db.case,
            seed.command.clone(),
            confirmation(&review),
        )
        .unwrap();

    assert_eq!(stored.capture.review, review);
    assert_eq!(stored.capture.review.command, seed.command);
    assert_eq!(
        stored.capture.review.resolved_values.purpose(),
        PrecautionaryHearingPurpose::Review
    );
    assert_eq!(
        stored.capture.review.resolved_values.review_targets(),
        seed.first
            .group
            .measures
            .iter()
            .map(reference)
            .collect::<Vec<_>>()
    );
    assert_eq!(stored.history.captures, vec![stored.capture.clone()]);
    assert_groups(&stored.history.measure_history, &[&seed.first]);
    assert_eq!(
        reads(&db, seed.actor)
            .get("session", db.case, seed.command.hearing_id, None)
            .unwrap(),
        stored
    );
}

#[test]
fn replacement_and_cancellation_preserve_exact_old_and_new_targets_after_later_decisions() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let original = reference(&seed.first.group.measures[0]);
    select_targets(&mut seed.command, vec![original]);
    let first = persist_hearing(&db, seed.actor.clone(), seed.command.clone());
    let original_bytes = precautionary_hearing_capture_bytes(&first.capture).unwrap();
    let confirmed = confirm_measures(
        &db,
        &seed,
        &seed
            .first
            .group
            .measures
            .iter()
            .map(reference)
            .collect::<Vec<_>>(),
    );
    let changed = reference(
        confirmed
            .group
            .measures
            .iter()
            .find(|row| row.result.id == original.id())
            .unwrap(),
    );
    assert_eq!(changed.id(), original.id());
    assert_eq!(original.revision().get(), 1);
    assert_eq!(changed.revision().get(), 2);
    let second = persist_hearing(
        &db,
        seed.actor.clone(),
        replace_targets(&first, vec![changed]),
    );
    let later = confirm_measures(
        &db,
        &seed,
        &confirmed
            .group
            .measures
            .iter()
            .map(reference)
            .collect::<Vec<_>>(),
    );
    let cancel = cancellation(&second);
    let no_admission = service_with_format(
        &db,
        seed.actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("cancellation must retain the previously admitted support")
        }))),
    );
    let cancel_review = no_admission
        .prepare("session", db.case, cancel.clone())
        .unwrap();
    let third = no_admission
        .submit("session", db.case, cancel, confirmation(&cancel_review))
        .unwrap();

    assert_eq!(
        first.capture.review.resolved_values.review_targets(),
        &[original]
    );
    assert_eq!(
        second.capture.review.resolved_values.review_targets(),
        &[changed]
    );
    assert_eq!(
        third.capture.review.resolved_values,
        second.capture.review.resolved_values
    );
    assert_eq!(third.capture.review.status, HearingStatus::Cancelled);
    assert_eq!(third.capture.review.result_revision.get(), 3);
    assert_eq!(third.history.origin, first.history.origin);
    assert_eq!(
        third.history.captures,
        vec![
            first.capture.clone(),
            second.capture.clone(),
            third.capture.clone()
        ]
    );
    assert_groups(&first.history.measure_history, &[&seed.first]);
    assert_groups(&second.history.measure_history, &[&seed.first, &confirmed]);
    assert_groups(&third.history.measure_history, &[&seed.first, &confirmed]);
    assert!(third
        .history
        .measure_history
        .groups
        .iter()
        .all(|entry| entry.origin.operation_id != later.origin.operation_id));
    for stored in [&first, &second, &third] {
        reopened(&db, &seed.actor, stored);
    }
    let query = reads(&db, seed.actor);
    same_operation(
        &query
            .get("session", db.case, seed.command.hearing_id, None)
            .unwrap(),
        &third,
    );
    let old = query
        .get(
            "session",
            db.case,
            seed.command.hearing_id,
            Some(first.capture.review.result_revision),
        )
        .unwrap();
    assert_eq!(
        precautionary_hearing_capture_bytes(&old.capture).unwrap(),
        original_bytes
    );
    let page = query
        .list("session", db.case, PrecautionaryHearingReadQuery::default())
        .unwrap();
    assert_eq!(page.items.len(), 1);
    same_operation(&page.items[0], &third);
    assert!(!page.has_more);
}

#[test]
fn fresh_review_accepts_an_exact_older_measure_after_a_later_revision_exists() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let original = reference(&seed.first.group.measures[0]);
    let later = confirm_measures(&db, &seed, &[original]);
    select_targets(&mut seed.command, vec![original]);

    let stored = persist_hearing(&db, seed.actor.clone(), seed.command.clone());

    assert_eq!(
        stored.capture.review.resolved_values.review_targets(),
        &[original]
    );
    assert_groups(&stored.history.measure_history, &[&seed.first]);
    assert!(stored
        .history
        .measure_history
        .groups
        .iter()
        .all(|entry| entry.origin.operation_id != later.origin.operation_id));
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn fresh_review_accepts_exact_revoked_and_ceased_measure_captures() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let terminal = crate::measure_fixture::persist(
        &db,
        seed.actor.clone(),
        measure_effects(
            &seed,
            vec![
                MeasureEffect::Revoke {
                    previous: reference(&seed.first.group.measures[0]),
                },
                MeasureEffect::Cease {
                    previous: reference(&seed.first.group.measures[1]),
                },
            ],
        ),
    );
    assert!(terminal
        .group
        .measures
        .iter()
        .any(|row| row.result.action == MeasureCaptureAction::Revoke));
    assert!(terminal
        .group
        .measures
        .iter()
        .any(|row| row.result.action == MeasureCaptureAction::Cease));
    let targets: Vec<_> = terminal.group.measures.iter().map(reference).collect();
    select_targets(&mut seed.command, targets.clone());

    let stored = persist_hearing(&db, seed.actor.clone(), seed.command.clone());

    assert_eq!(
        stored.capture.review.resolved_values.review_targets(),
        targets
    );
    assert_groups(&stored.history.measure_history, &[&seed.first, &terminal]);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn one_selected_measure_retains_its_owners_sibling_and_that_siblings_independent_ancestors() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let mut independent = crate::measure_fixture::fresh(&seed.measure_command);
    independent.outcome = crate::measure_fixture::impositions(
        &seed.first.group.measures[0].result.sources.subject,
        2,
    );
    let other = crate::measure_fixture::persist(&db, seed.actor.clone(), independent);
    let selected_id = seed.first.group.measures[0].result.id;
    let joined = confirm_measures(
        &db,
        &seed,
        &[
            reference(&seed.first.group.measures[0]),
            reference(&other.group.measures[0]),
        ],
    );
    let selected = reference(
        joined
            .group
            .measures
            .iter()
            .find(|row| row.result.id == selected_id)
            .unwrap(),
    );
    select_targets(&mut seed.command, vec![selected]);

    let stored = persist_hearing(&db, seed.actor.clone(), seed.command.clone());

    assert_eq!(
        stored.capture.review.resolved_values.review_targets(),
        &[selected]
    );
    assert_groups(
        &stored.history.measure_history,
        &[&seed.first, &other, &joined],
    );
    assert_eq!(
        stored
            .history
            .measure_history
            .groups
            .iter()
            .map(|entry| entry.capture.measures.len())
            .sum::<usize>(),
        6
    );
    reopened(&db, &seed.actor, &stored);
}
