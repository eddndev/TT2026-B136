use super::*;

fn remove_member(db: &mut Fixture, capture: &MeasureCapture) {
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_measure_revisions DISABLE TRIGGER ALL")
        .unwrap();
    assert_eq!(
        tx.execute(
            "DELETE FROM case_measure_revisions WHERE measure_id=$1 AND revision=$2",
            &[
                &capture.result.id.as_uuid(),
                &i64::from(capture.result.revision.get())
            ],
        )
        .unwrap(),
        1
    );
    tx.batch_execute("ALTER TABLE case_measure_revisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();
}

#[test]
fn stale_exact_predecessor_rejects_while_original_receipts_remain_readable_and_replayable() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let stale = command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: reference(&first.group.measures[0]),
        }],
    );
    let workflow = service(&db, seed.actor.clone());
    let prepared = workflow.prepare("session", db.case, stale.clone()).unwrap();
    let advanced = persist(
        &db,
        seed.actor.clone(),
        command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(&first.group.measures[0]),
            }],
        ),
    );
    let before = snapshot(&mut db);

    assert!(workflow.prepare("session", db.case, stale.clone()).is_err());
    assert!(workflow
        .submit("session", db.case, stale, confirmation(&prepared))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    reopened(&db, &seed.actor, &first);
    reopened(&db, &seed.actor, &advanced);
}

#[test]
fn lost_latest_member_cannot_restore_an_older_head_or_hide_the_committed_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let latest = persist(
        &db,
        seed.actor.clone(),
        command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(&first.group.measures[0]),
            }],
        ),
    );
    let storage = store(&db);
    let query = reads(&db, seed.actor.clone());
    let workflow = service(&db, seed.actor.clone());
    remove_member(&mut db, &latest.group.measures[0]);
    let before = snapshot(&mut db);

    assert!(storage
        .get(&seed.actor, db.case, latest.origin.decision_id)
        .is_err());
    assert!(storage
        .get_operation(&seed.actor, db.case, latest.origin.operation_id)
        .is_err());
    assert!(query
        .get("session", db.case, latest.origin.decision_id)
        .is_err());
    assert!(query
        .list("session", db.case, MeasureDecisionReadQuery::default())
        .is_err());
    for previous in [&first.group.measures[0], &latest.group.measures[0]] {
        let fresh = command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(previous),
            }],
        );
        assert!(workflow.prepare("session", db.case, fresh).is_err());
    }
    assert_eq!(snapshot(&mut db), before);
    let retained: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_measure_decisions d JOIN case_measure_operations o \
         ON o.operation_id=d.operation_id JOIN audit_events a ON a.sequence=o.audit_sequence \
         WHERE d.operation_id=$1",
            &[&latest.origin.operation_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(retained, 1);
}

#[test]
fn missing_sibling_ancestor_rejects_even_when_the_selected_members_own_branch_survives() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let mut independent = fresh(&seed.command);
    independent.outcome = impositions(&seed.subject, 2);
    let second = persist(&db, seed.actor.clone(), independent);
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
    let selected = member(&joined, first.group.measures[0].result.id);
    let next = command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: reference(selected),
        }],
    );
    let storage = store(&db);
    let workflow = service(&db, seed.actor.clone());
    remove_member(&mut db, &second.group.measures[0]);
    let before = snapshot(&mut db);

    assert!(storage
        .get(&seed.actor, db.case, joined.origin.decision_id)
        .is_err());
    assert!(storage
        .get_operation(&seed.actor, db.case, joined.origin.operation_id)
        .is_err());
    assert!(workflow.prepare("session", db.case, next).is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn substituted_no_change_outcome_cannot_hide_a_lost_latest_member_from_live_admission() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let latest = persist(
        &db,
        seed.actor.clone(),
        command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(&first.group.measures[0]),
            }],
        ),
    );
    let unrelated = persist(&db, seed.actor.clone(), no_change(&seed.command));
    let storage = store(&db);
    let workflow = service(&db, seed.actor.clone());
    let stale = command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: reference(&first.group.measures[0]),
        }],
    );
    let latest_operation = latest.origin.operation_id.as_uuid();
    let retained_query = "SELECT jsonb_build_object(
        'decision',to_jsonb(d)-ARRAY['outcome_canonical','outcome_view','outcome_digest'],
        'operation',to_jsonb(o),'audit',to_jsonb(a))
        FROM case_measure_decisions d
        JOIN case_measure_operations o ON o.operation_id=d.operation_id
        JOIN audit_events a ON a.sequence=o.audit_sequence WHERE d.operation_id=$1";
    let retained: serde_json::Value = db
        .admin
        .query_one(retained_query, &[&latest_operation])
        .unwrap()
        .get(0);
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE case_measure_decisions DISABLE TRIGGER ALL;
         ALTER TABLE case_measure_revisions DISABLE TRIGGER ALL",
    )
    .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE case_measure_decisions d
         SET outcome_canonical=source.outcome_canonical,outcome_view=source.outcome_view,
             outcome_digest=source.outcome_digest
         FROM case_measure_decisions source
         WHERE d.operation_id=$1 AND source.operation_id=$2",
            &[&latest_operation, &unrelated.origin.operation_id.as_uuid()],
        )
        .unwrap(),
        1
    );
    assert_eq!(
        tx.execute(
            "DELETE FROM case_measure_revisions WHERE owner_operation=$1",
            &[&latest_operation],
        )
        .unwrap(),
        1
    );
    tx.batch_execute(
        "ALTER TABLE case_measure_decisions ENABLE TRIGGER ALL;
         ALTER TABLE case_measure_revisions ENABLE TRIGGER ALL",
    )
    .unwrap();
    tx.commit().unwrap();
    let after_damage: serde_json::Value = db
        .admin
        .query_one(retained_query, &[&latest_operation])
        .unwrap()
        .get(0);
    assert_eq!(after_damage, retained);
    let before = snapshot(&mut db);

    assert!(storage
        .prepare(
            &seed.actor,
            db.case,
            &stale,
            &application::documents::StageSupportReadLimits::default(),
        )
        .is_err());
    assert!(workflow.prepare("session", db.case, stale).is_err());
    assert_eq!(snapshot(&mut db), before);
}
