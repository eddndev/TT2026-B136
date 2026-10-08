use super::*;
use application::documents::StageSupportReadLimits;

#[test]
fn losing_either_replacement_sibling_root_or_original_audit_rejects_the_whole_owner() {
    for missing in ["marked", "replacement", "root", "audit"] {
        let Some(mut db) = Fixture::new() else { return };
        let (seed, _, _, command) = setup_joint(&mut db);
        let original = persist(&db, seed.actor.clone(), command);
        reopened(&db, &seed.actor, &original);
        let storage = store(&db);
        let records = crate::measure_fixture::store(&db);
        let workflow = replay_service(&db, seed.actor.clone(), storage.clone());
        let new = replacement_row(&original);
        let old_id = original.capture.review.result.id;
        let new_id = new.result.id;
        let op = original.origin.operation_id;
        let statement = match missing {
            "marked" => format!("DELETE FROM case_measure_revisions WHERE measure_id='{old_id}' AND owner_operation='{op}'"),
            "replacement" => format!("DELETE FROM case_measure_revisions WHERE measure_id='{new_id}'"),
            "root" => format!("DELETE FROM case_measures WHERE id='{new_id}'"),
            "audit" => format!("DELETE FROM audit_events WHERE sequence=(SELECT audit_sequence FROM case_measure_operations WHERE operation_id='{op}')"),
            _ => unreachable!(),
        };
        damage(&mut db, &statement);
        let before = snapshot(&mut db);
        reject_original(&db, &workflow, &original);
        for id in [old_id, new_id] {
            assert!(
                MeasureRecordReadStore::get(records.as_ref(), &seed.actor, db.case, id).is_err()
            );
        }
        let mut reuse = crate::measure_fixture::fresh(&seed.command);
        reuse.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(MeasureProposal {
                id: new_id,
                values: new.result.values.clone(),
            }),
        ]))
        .unwrap();
        assert!(MeasureDecisionRecordStore::prepare(
            records.as_ref(),
            &seed.actor,
            db.case,
            &reuse,
            &StageSupportReadLimits::standard()
        )
        .is_err());
        assert!(PostgresMeasureAdministrativeStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at))
        )
        .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn missing_latest_revision_of_the_replacement_cannot_reveal_its_initial_valid_record() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, _, command) = setup_joint(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    let new = replacement_row(&original);
    let latest = persist(
        &db,
        seed.actor.clone(),
        correction(
            row_reference(new),
            seed.command.context,
            &new.result.values,
            "Latest replacement wording",
        ),
    );
    reopened(&db, &seed.actor, &latest);
    let records = crate::measure_fixture::store(&db);
    let storage = store(&db);
    let workflow = replay_service(&db, seed.actor.clone(), storage.clone());
    damage(
        &mut db,
        &format!(
            "DELETE FROM case_measure_revisions WHERE measure_id='{}' AND revision=2",
            new.result.id
        ),
    );
    let before = snapshot(&mut db);
    assert!(
        MeasureRecordReadStore::get(records.as_ref(), &seed.actor, db.case, new.result.id).is_err()
    );
    assert!(storage
        .prepare(
            &seed.actor,
            db.case,
            &mark(row_reference(new), seed.command.context),
            &StageSupportReadLimits::standard()
        )
        .is_err());
    reject_original(&db, &workflow, &latest);
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn altered_replacement_subject_provenance_rejects_replay_and_both_owned_records() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, subject, command) = setup_joint(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    let storage = store(&db);
    let records = crate::measure_fixture::store(&db);
    let workflow = replay_service(&db, seed.actor.clone(), storage);
    reopened(&db, &seed.actor, &original);
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_subject_revisions DISABLE TRIGGER ALL")
        .unwrap();
    tx.execute(
        "UPDATE case_subject_revisions SET changed_by_email='altered-new-subject@example.test'
        WHERE subject_id=$1 AND revision=$2",
        &[&subject.id.as_uuid(), &i64::from(subject.revision.get())],
    )
    .unwrap();
    tx.batch_execute("ALTER TABLE case_subject_revisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();
    let before = snapshot(&mut db);
    reject_original(&db, &workflow, &original);
    for row in &original.capture.records {
        assert!(MeasureRecordReadStore::exact(
            records.as_ref(),
            &seed.actor,
            db.case,
            row_reference(row)
        )
        .is_err());
    }
    assert_eq!(snapshot(&mut db), before);
}

fn damage(db: &mut Fixture, statement: &str) {
    let tables = [
        "case_measure_operations",
        "case_measure_administrations",
        "case_measure_revisions",
        "case_measures",
        "audit_events",
    ];
    let mut tx = db.admin.transaction().unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.batch_execute(statement).unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.commit().unwrap();
}

fn replay_service(
    db: &Fixture,
    actor: Principal,
    storage: Arc<PostgresMeasureAdministrativeStore>,
) -> MeasureAdministrativeService {
    MeasureAdministrativeService::new(
        storage,
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(FormatCheck(Some(Box::new(|| {
            panic!("replay must not admit documentary support")
        })))),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn reject_original(
    db: &Fixture,
    workflow: &MeasureAdministrativeService,
    original: &MeasureAdministrativeStoredOperation,
) {
    let command = &original.capture.review.command;
    assert!(workflow
        .prepare("session", db.case, command.clone())
        .is_err());
    assert!(workflow
        .submit(
            "session",
            db.case,
            command.clone(),
            confirmation(&original.capture.review)
        )
        .is_err());
}
