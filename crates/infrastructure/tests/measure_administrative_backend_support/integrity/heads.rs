use super::*;

#[test]
fn judicial_and_administrative_operations_share_one_exact_uuid_namespace() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let mut collision = command.clone();
    collision.operation_id =
        MeasureCorrectionOperationId::from_uuid(judicial.origin.operation_id.as_uuid());
    let before = inventory_snapshot(&mut db.admin);
    assert!(matches!(
        workflow.prepare("session", db.case, collision),
        Err(ApplicationError::MeasureAdministrative(
            MeasureAdministrativeError::OperationConflict
        ))
    ));
    assert_eq!(inventory_snapshot(&mut db.admin), before);

    let administrative = persist(&db, seed.actor.clone(), command);
    let mut collision = crate::measure_fixture::no_change(&seed.command);
    collision.operation_id = MeasureDecisionOperationId::from_uuid(
        administrative.capture.review.command.operation_id.as_uuid(),
    );
    let judicial_workflow = crate::measure_fixture::service(&db, seed.actor);
    let before = inventory_snapshot(&mut db.admin);
    assert!(judicial_workflow
        .prepare("session", db.case, collision)
        .is_err());
    assert_eq!(inventory_snapshot(&mut db.admin), before);
}

#[test]
fn later_judicial_and_administrative_records_both_make_the_exact_old_head_stale() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, first_command) = setup(&mut db);
    let second = &judicial.group.measures[1];
    let second_command = correction(
        reference(second),
        seed.command.context,
        &second.result.values,
        "Second correction",
    );
    let workflow = service(&db, seed.actor.clone());
    let first_review = workflow
        .prepare("session", db.case, first_command.clone())
        .unwrap();
    let second_review = workflow
        .prepare("session", db.case, second_command.clone())
        .unwrap();
    crate::measure_fixture::persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: first_command.target,
            }],
        ),
    );
    persist(&db, seed.actor.clone(), second_command.clone());
    let mut stale_second = second_command;
    stale_second.operation_id = MeasureCorrectionOperationId::new();
    let before = inventory_snapshot(&mut db.admin);
    for (command, review) in [(first_command, first_review), (stale_second, second_review)] {
        assert!(matches!(
            workflow.prepare("session", db.case, command.clone()),
            Err(ApplicationError::MeasureAdministrative(
                MeasureAdministrativeError::StaleHead
            ))
        ));
        assert!(workflow
            .submit("session", db.case, command, confirmation(&review))
            .is_err());
    }
    assert_eq!(inventory_snapshot(&mut db.admin), before);
    assert_eq!(
        crate::measure_fixture::reads(&db, seed.actor)
            .get("session", db.case, judicial.origin.decision_id)
            .unwrap()
            .group,
        judicial.group,
    );
}

#[test]
fn an_entered_in_error_head_rejects_both_correction_and_another_mark() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, _) = setup(&mut db);
    let marked = persist(
        &db,
        seed.actor.clone(),
        mark(reference(&judicial.group.measures[0]), seed.command.context),
    );
    let target = corrected_reference(&marked.capture);
    let correction = correction(
        target,
        seed.command.context,
        &marked.capture.review.result.values,
        "Further correction",
    );
    let workflow = service(&db, seed.actor.clone());
    let before = inventory_snapshot(&mut db.admin);
    for command in [correction, mark(target, seed.command.context)] {
        assert!(workflow.prepare("session", db.case, command).is_err());
    }
    assert_eq!(inventory_snapshot(&mut db.admin), before);
    reopened(&db, &seed.actor, &marked);
}
