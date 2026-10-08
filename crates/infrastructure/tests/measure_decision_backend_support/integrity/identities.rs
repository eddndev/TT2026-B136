use super::*;

#[test]
fn original_operation_decision_and_measure_ids_cannot_own_another_group() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let workflow = service(&db, seed.actor.clone());
    let before = snapshot(&mut db);
    for identity in 0..3 {
        let mut command = fresh(&seed.command);
        command.outcome = impositions(&seed.subject, 1);
        match identity {
            0 => command.operation_id = seed.command.operation_id,
            1 => command.decision_id = seed.command.decision_id,
            _ => {
                command.outcome =
                    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
                        MeasureEffect::Impose(MeasureProposal {
                            id: original.group.measures[0].result.id,
                            values: values(&seed.subject),
                        }),
                    ]))
                    .unwrap();
            }
        }
        assert!(
            workflow.prepare("session", db.case, command).is_err(),
            "reused identity {identity}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn surviving_decision_outcome_reserves_a_measure_id_after_its_root_and_row_are_lost() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let other = setup(&mut db);
    let storage = store(&db);
    let id = original.group.measures[0].result.id;
    damage(&mut db, &format!(
        "DELETE FROM case_measure_revisions WHERE measure_id='{id}'; DELETE FROM case_measures WHERE id='{id}'",
    ));
    let mut command = other.command.clone();
    command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id,
            values: values(&other.subject),
        }),
    ]))
    .unwrap();
    let before = snapshot(&mut db);
    assert!(MeasureDecisionStore::prepare(
        storage.as_ref(),
        &other.actor,
        db.case,
        &command,
        &StageSupportReadLimits::default()
    )
    .is_err());
    assert_eq!(snapshot(&mut db), before);
}
