use super::*;
use application::{documents::StageSupportReadLimits, ApplicationError};
use domain::crypto::Sha256Digest;
use postgres::{types::ToSql, Client, NoTls};
use std::sync::atomic::{AtomicBool, Ordering};

fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn damage(client: &mut Client, table: &str, statement: &str, parameters: &[&(dyn ToSql + Sync)]) {
    let mut tx = client.transaction().unwrap();
    tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
        .unwrap();
    assert_eq!(tx.execute(statement, parameters).unwrap(), 1);
    tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
        .unwrap();
    tx.commit().unwrap();
}

fn reject_original(
    storage: &PostgresMeasureDecisionStore,
    actor: &Principal,
    case: CaseId,
    original: &MeasureDecisionStoredOperation,
) {
    assert!(
        MeasureDecisionReadStore::get(storage, actor, case, original.origin.decision_id).is_err()
    );
    assert!(MeasureDecisionReadStore::get_operation(
        storage,
        actor,
        case,
        original.origin.operation_id
    )
    .is_err());
    assert!(MeasureDecisionStore::prepare(
        storage,
        actor,
        case,
        &original.group.review.command,
        &StageSupportReadLimits::default(),
    )
    .is_err());
}

fn hidden_prefix(
    db: &mut Fixture,
) -> (
    Seed,
    MeasureDecisionStoredOperation,
    PrecautionaryHearingStoredOperation,
    MeasureDecisionStoredOperation,
) {
    let seed = crate::measure_fixture::setup(db);
    let first = persist(db, seed.actor.clone(), seed.command.clone());
    let hearing_one = crate::hearing_fixture::persist(
        db,
        seed.actor.clone(),
        hearing_command(db, &seed, vec![reference(&first.group.measures[0])]),
    );
    let hearing_two = crate::hearing_fixture::persist(
        db,
        seed.actor.clone(),
        replace_targets(&hearing_one, vec![]),
    );
    let mut command = no_change(&seed.command);
    command.anchor = Some(anchor(&hearing_two));
    let stored = persist(db, seed.actor.clone(), command);
    assert!(stored.measure_history.groups.is_empty());
    assert_groups(&hearing_two.history.measure_history, &[&first]);
    assert_anchor(&stored, &hearing_two);
    (seed, first, hearing_two, stored)
}

#[test]
fn missing_earlier_review_revision_invalidates_an_imposition_anchor_with_empty_wire_history() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, hearing, stored) = hidden_prefix(&mut db);
    let storage = store(&db);
    assert_eq!(
        MeasureDecisionReadStore::get(
            storage.as_ref(),
            &seed.actor,
            db.case,
            stored.origin.decision_id
        )
        .unwrap(),
        stored
    );
    damage(
        &mut db.admin,
        "case_precautionary_hearing_revisions",
        "DELETE FROM case_precautionary_hearing_revisions WHERE hearing_id=$1 AND revision=1",
        &[&hearing.capture.review.command.hearing_id.as_uuid()],
    );
    let before = snapshot(&mut db);

    reject_original(&storage, &seed.actor, db.case, &stored);
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(db.admin.query_one(
        "SELECT count(*) FROM case_precautionary_hearing_revisions WHERE hearing_id=$1 AND revision=2",
        &[&hearing.capture.review.command.hearing_id.as_uuid()],
    ).unwrap().get::<_, i64>(0), 1);
}

#[test]
fn missing_prefix_only_group_invalidates_the_later_imposition_anchor() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, first, hearing, stored) = hidden_prefix(&mut db);
    let storage = store(&db);
    assert_eq!(
        MeasureDecisionReadStore::get(
            storage.as_ref(),
            &seed.actor,
            db.case,
            stored.origin.decision_id
        )
        .unwrap(),
        stored
    );
    damage(
        &mut db.admin,
        "case_measure_decisions",
        "DELETE FROM case_measure_decisions WHERE operation_id=$1",
        &[&first.origin.operation_id.as_uuid()],
    );
    let before = snapshot(&mut db);

    reject_original(&storage, &seed.actor, db.case, &stored);
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM case_precautionary_hearing_revisions WHERE hearing_id=$1",
                &[&hearing.capture.review.command.hearing_id.as_uuid()],
            )
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}

#[test]
fn exact_precautionary_anchor_requires_the_original_capture_digest_and_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.seed.actor);
    workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    for changed in ["digest", "revision"] {
        let mut command = fresh(&seed.command);
        let Some(MeasureDecisionAnchorRef::Precautionary {
            revision,
            capture_digest,
            ..
        }) = &mut command.anchor
        else {
            unreachable!()
        };
        if changed == "digest" {
            let mut bytes = *capture_digest.as_bytes();
            bytes[0] ^= 1;
            *capture_digest = Sha256Digest::from_array(bytes);
        } else {
            *revision = PrecautionaryHearingRevision::new(2).unwrap();
        }
        let before = snapshot(&mut db);

        assert!(
            workflow.prepare("session", db.case, command).is_err(),
            "accepted {changed}"
        );

        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn a_real_precautionary_anchor_from_another_case_is_rejected_without_writes() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let selected_case = db.case;
    let foreign = setup(&mut db);
    assert_ne!(foreign.hearing.capture.review.case_id, selected_case);
    db.case = selected_case;
    let workflow = service(&db, seed.seed.actor);
    workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    seed.command.anchor = Some(anchor(&foreign.hearing));
    let before = snapshot(&mut db);

    assert!(workflow.prepare("session", db.case, seed.command).is_err());

    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn an_anchor_target_requires_its_unselected_owners_sibling() {
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
    assert_groups(&stored.measure_history, &[&first]);
    let storage = store(&db);
    damage(
        &mut db.admin,
        "case_measure_revisions",
        "DELETE FROM case_measure_revisions WHERE measure_id=$1 AND revision=1",
        &[&first.group.measures[1].result.id.as_uuid()],
    );
    let before = snapshot(&mut db);

    reject_original(&storage, &seed.actor, db.case, &stored);
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM case_measure_revisions WHERE measure_id=$1 AND revision=1",
                &[&first.group.measures[0].result.id.as_uuid()],
            )
            .unwrap()
            .get::<_, i64>(0),
        1
    );
}

#[test]
fn losing_the_anchor_root_during_admission_rejects_every_decision_and_audit_write() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let reviewed = service(&db, seed.seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let url = db.admin_url.clone();
    let id = seed.hearing.capture.review.command.hearing_id.as_uuid();
    let admitted = Arc::new(AtomicBool::new(false));
    let observed = admitted.clone();
    let workflow = service_with_format(
        &db,
        seed.seed.actor,
        FormatCheck(Some(Box::new(move || {
            let mut client = Client::connect(&url, NoTls).unwrap();
            damage(
                &mut client,
                "case_precautionary_hearings",
                "DELETE FROM case_precautionary_hearings WHERE id=$1",
                &[&id],
            );
            observed.store(true, Ordering::SeqCst);
        }))),
    );
    let before = snapshot(&mut db);

    assert!(workflow
        .submit("session", db.case, seed.command, confirmation(&reviewed))
        .is_err());

    assert!(admitted.load(Ordering::SeqCst));
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM case_precautionary_hearings WHERE id=$1",
                &[&id],
            )
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}

#[test]
fn changed_original_hearing_mutation_audit_invalidates_an_unchanged_later_anchor() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, hearing, stored) = hidden_prefix(&mut db);
    let storage = store(&db);
    let sequence: i64 = db.admin.query_one(
        "SELECT audit_sequence FROM case_precautionary_hearing_revisions WHERE hearing_id=$1 AND revision=1",
        &[&hearing.capture.review.command.hearing_id.as_uuid()],
    ).unwrap().get(0);
    damage(
        &mut db.admin,
        "audit_events",
        "UPDATE audit_events SET actor='changed-original-actor@example.test' WHERE sequence=$1",
        &[&sequence],
    );
    let before = snapshot(&mut db);

    reject_original(&storage, &seed.actor, db.case, &stored);
    assert!(open(&db).is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn malformed_latest_hearing_digest_does_not_fall_back_to_an_older_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let second = crate::hearing_fixture::persist(
        &db,
        seed.seed.actor.clone(),
        replace_targets(&seed.hearing, vec![]),
    );
    let id = second.capture.review.command.hearing_id;
    let query = crate::hearing_fixture::reads(&db, seed.seed.actor);
    assert_eq!(
        query.get("session", db.case, id, None).unwrap().capture,
        second.capture
    );
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute(
        "ALTER TABLE case_precautionary_hearing_revisions
         DROP CONSTRAINT precautionary_hearing_capture_size;
         ALTER TABLE case_precautionary_hearing_revisions DISABLE TRIGGER ALL",
    )
    .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE case_precautionary_hearing_revisions SET capture_digest=$1
             WHERE hearing_id=$2 AND revision=2",
            &[&vec![1_u8], &id.as_uuid()],
        )
        .unwrap(),
        1
    );
    tx.batch_execute("ALTER TABLE case_precautionary_hearing_revisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();
    let before = snapshot(&mut db);

    assert!(query.get("session", db.case, id, None).is_err());

    assert_eq!(snapshot(&mut db), before);
}
