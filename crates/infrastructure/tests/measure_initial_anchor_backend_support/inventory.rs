use super::*;
use application::documents::StageSupportReadLimits;

#[test]
fn anchored_surviving_outcome_reserves_identity_after_root_and_latest_member_loss() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original_case = db.case;
    let original = persist(&db, seed.actor.clone(), seed.command);
    assert_anchor(&original, &seed.hearing);
    let other = crate::measure_fixture::setup(&mut db);
    let storage = store(&db);
    assert_eq!(
        storage
            .get(&seed.actor, original_case, original.origin.decision_id)
            .unwrap(),
        original
    );
    let id = original.group.measures[0].result.id;
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE case_measures DISABLE TRIGGER ALL;
         ALTER TABLE case_measure_revisions DISABLE TRIGGER ALL",
    )
    .unwrap();
    assert_eq!(
        tx.execute(
            "DELETE FROM case_measure_revisions WHERE measure_id=$1",
            &[&id.as_uuid()]
        )
        .unwrap(),
        1
    );
    assert_eq!(
        tx.execute("DELETE FROM case_measures WHERE id=$1", &[&id.as_uuid()])
            .unwrap(),
        1
    );
    tx.batch_execute(
        "ALTER TABLE case_measures ENABLE TRIGGER ALL;
         ALTER TABLE case_measure_revisions ENABLE TRIGGER ALL",
    )
    .unwrap();
    tx.commit().unwrap();
    let mut command = other.command;
    command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id,
            values: values(&other.subject),
        }),
    ]))
    .unwrap();
    let before = snapshot(&mut db);

    assert!(storage
        .prepare(
            &other.actor,
            db.case,
            &command,
            &StageSupportReadLimits::default()
        )
        .is_err());

    assert_eq!(snapshot(&mut db), before);
    let retained: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_measure_decisions d
         JOIN case_measure_operations o ON o.operation_id=d.operation_id
         JOIN audit_events a ON a.sequence=o.audit_sequence
         WHERE d.operation_id=$1 AND d.anchor_kind='initial'",
            &[&original.origin.operation_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(retained, 1);
}

#[test]
fn another_real_anchor_cannot_replace_stored_selectors_without_changing_original_commitments() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let replacement = super::history::replace(&db, &seed.hearing);
    let storage = store(&db);
    assert_eq!(
        storage
            .get(&seed.actor, db.case, original.origin.decision_id)
            .unwrap(),
        original
    );
    let operation = original.origin.operation_id.as_uuid();
    let retained_query = "SELECT jsonb_build_object(
        'decision',to_jsonb(d)-ARRAY['anchor_kind','anchor_hearing_id','anchor_revision',
            'anchor_values_digest','anchor_submission_digest'],
        'operation',to_jsonb(o),'audit',to_jsonb(a))
        FROM case_measure_decisions d
        JOIN case_measure_operations o ON o.operation_id=d.operation_id
        JOIN audit_events a ON a.sequence=o.audit_sequence WHERE d.operation_id=$1";
    let retained: serde_json::Value = db
        .admin
        .query_one(retained_query, &[&operation])
        .unwrap()
        .get(0);
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_measure_decisions DISABLE TRIGGER ALL")
        .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE case_measure_decisions SET anchor_revision=$2,anchor_values_digest=$3,
            anchor_submission_digest=$4 WHERE operation_id=$1",
            &[
                &operation,
                &i64::from(replacement.snapshot.revision.get()),
                &replacement.snapshot.values_digest.as_bytes().as_slice(),
                &replacement
                    .snapshot
                    .receipt
                    .submission_digest
                    .as_bytes()
                    .as_slice()
            ],
        )
        .unwrap(),
        1
    );
    tx.batch_execute("ALTER TABLE case_measure_decisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();
    let after_damage: serde_json::Value = db
        .admin
        .query_one(retained_query, &[&operation])
        .unwrap()
        .get(0);
    assert_eq!(after_damage, retained);
    let mut unrelated = no_change(&seed.command);
    unrelated.anchor = None;
    let before = snapshot(&mut db);

    assert!(storage
        .prepare(
            &seed.actor,
            db.case,
            &unrelated,
            &StageSupportReadLimits::default()
        )
        .is_err());
    assert!(storage
        .get(&seed.actor, db.case, original.origin.decision_id)
        .is_err());
    assert!(storage
        .get_operation(&seed.actor, db.case, original.origin.operation_id)
        .is_err());
    assert!(storage
        .prepare(
            &seed.actor,
            db.case,
            &seed.command,
            &StageSupportReadLimits::default()
        )
        .is_err());

    assert_eq!(snapshot(&mut db), before);
}
