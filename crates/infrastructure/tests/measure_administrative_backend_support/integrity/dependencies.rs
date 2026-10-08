use super::*;

#[test]
fn a_historical_review_still_blocks_correction_after_imposition_replacement_and_cancel() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let scheduled = review_hearing(&db, &seed, vec![command.target]);
    known_dependants(&mut db, &workflow, &command, &draft);

    let replaced = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        replace_with_imposition(&scheduled),
    );
    assert_eq!(
        replaced.capture.review.resolved_values.purpose(),
        PrecautionaryHearingPurpose::Imposition
    );
    assert!(replaced
        .capture
        .review
        .resolved_values
        .review_targets()
        .is_empty());
    known_dependants(&mut db, &workflow, &command, &draft);

    let cancelled = crate::hearing_fixture::persist(
        &db,
        seed.actor,
        crate::hearing_fixture::cancellation(&replaced),
    );
    assert_eq!(cancelled.history.captures.len(), 3);
    assert_eq!(cancelled.history.captures[0], scheduled.capture);
    known_dependants(&mut db, &workflow, &command, &draft);
}

#[test]
fn a_zero_member_decision_retains_its_exact_review_anchor_dependency() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let hearing = review_hearing(&db, &seed, vec![command.target]);
    let mut judicial_command = crate::measure_fixture::no_change(&seed.command);
    judicial_command.anchor = Some(anchor(&hearing));
    let zero = crate::measure_fixture::persist(&db, seed.actor, judicial_command);
    assert!(zero.group.measures.is_empty());
    let mut judicial = zero.measure_history.clone();
    judicial.groups.push(MeasureGroupEvidence {
        origin: zero.origin.clone(),
        capture: zero.group.clone(),
    });
    let inventory = MeasureAdministrativeDependencyInventory {
        records: MeasureDecisionRecordHistoryEvidence {
            records: MeasureRecordHistoryEvidence {
                judicial,
                administrative: vec![],
            },
            decisions: vec![],
        },
        hearings: vec![MeasureAdministrativeHearingHistory {
            origin: hearing.history.origin,
            captures: hearing.history.captures,
        }],
    };
    let checked = inspect_measure_administrative_dependencies(
        &RingSha256Hasher,
        db.case,
        command.target,
        &inventory,
    )
    .unwrap();
    assert!(checked.dependants().iter().any(|edge| matches!(edge,
        MeasureAdministrativeDependant::ReviewAnchor {
            owner: MeasureDependencyJudicialOwner::V1(owner), target, ..
        } if owner.operation_id == zero.origin.operation_id && *target == command.target
    )));
    known_dependants(&mut db, &workflow, &command, &draft);
}

#[test]
fn review_and_zero_member_anchor_of_a_sibling_do_not_block_the_selected_measure() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let sibling = reference(&judicial.group.measures[1]);
    assert_ne!(sibling.id(), command.target.id());
    let hearing = review_hearing(&db, &seed, vec![sibling]);
    let mut no_change = crate::measure_fixture::no_change(&seed.command);
    no_change.anchor = Some(anchor(&hearing));
    let dependant = crate::measure_fixture::persist(&db, seed.actor.clone(), no_change);

    let stored = persist(&db, seed.actor.clone(), command);

    assert_eq!(
        stored.capture.review.result.previous,
        reference(&judicial.group.measures[0])
    );
    assert_eq!(stored.record_history.records.judicial.groups.len(), 1);
    assert!(stored
        .record_history
        .records
        .judicial
        .groups
        .iter()
        .all(|group| group.origin.operation_id != dependant.origin.operation_id));
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn review_of_an_older_revision_does_not_block_correction_of_the_later_exact_head() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, first_command) = setup(&mut db);
    let original = reference(&judicial.group.measures[0]);
    review_hearing(&db, &seed, vec![original]);
    let later = crate::measure_fixture::persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm { previous: original }],
        ),
    );
    let current = &later.group.measures[0];
    let command = correction(
        reference(current),
        first_command.context,
        &current.result.values,
        "Correct the later revision",
    );

    let stored = persist(&db, seed.actor.clone(), command);

    assert_eq!(stored.capture.review.result.previous.id(), original.id());
    assert_eq!(stored.capture.review.result.previous.revision().get(), 2);
    assert_eq!(stored.capture.review.result.revision.get(), 3);
    assert_eq!(
        stored.capture.review.result.last_judicial.reference,
        reference(current)
    );
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn original_administrative_replay_survives_a_later_review_of_its_judicial_predecessor() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), command.clone());
    let hearing = review_hearing(&db, &seed, vec![command.target]);
    assert_eq!(
        hearing.capture.review.resolved_values.review_targets(),
        &[command.target]
    );

    reopened(&db, &seed.actor, &original);

    assert_eq!(original.record_history.records.judicial.groups.len(), 1);
    assert!(original.record_history.records.administrative.is_empty());
}
