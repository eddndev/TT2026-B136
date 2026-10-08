use super::*;
use domain::crypto::Sha256Digest;
use std::sync::atomic::{AtomicBool, Ordering};

fn damage_member(db: &mut Fixture, capture: &MeasureCapture, remove: bool) {
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_measure_revisions DISABLE TRIGGER ALL")
        .unwrap();
    let statement = if remove {
        "DELETE FROM case_measure_revisions WHERE measure_id=$1 AND revision=$2"
    } else {
        "UPDATE case_measure_revisions
         SET values_view=jsonb_set(values_view,'{conditions}',to_jsonb('Altered stored conditions'::text))
         WHERE measure_id=$1 AND revision=$2"
    };
    assert_eq!(
        tx.execute(
            statement,
            &[
                &capture.result.id.as_uuid(),
                &i64::from(capture.result.revision.get()),
            ]
        )
        .unwrap(),
        1
    );
    tx.batch_execute("ALTER TABLE case_measure_revisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();
}

#[test]
fn foreign_case_measure_target_rejects_without_any_hearing_or_audit_write() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let selected_case = db.case;
    let foreign_seed = crate::measure_fixture::setup(&mut db);
    let foreign = crate::measure_fixture::persist(&db, foreign_seed.actor, foreign_seed.command);
    assert_ne!(foreign.group.review.case_id, selected_case);
    db.case = selected_case;
    select_targets(
        &mut seed.command,
        vec![reference(&foreign.group.measures[0])],
    );
    let workflow = service(&db, seed.actor);
    let before = snapshot(&mut db);

    assert!(workflow.prepare("session", db.case, seed.command).is_err());

    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn conflicting_exact_target_digest_rejects_without_any_hearing_or_audit_write() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let selected = reference(&seed.first.group.measures[0]);
    let mut digest = *selected.digest().as_bytes();
    digest[0] ^= 1;
    select_targets(
        &mut seed.command,
        vec![PrecautionaryMeasureRef::new(
            selected.id(),
            selected.revision(),
            Sha256Digest::from_array(digest),
        )],
    );
    let workflow = service(&db, seed.actor);
    let before = snapshot(&mut db);

    assert!(workflow.prepare("session", db.case, seed.command).is_err());

    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn absent_or_corrupt_actual_ancestor_rejects_even_when_selected_owner_remains_complete() {
    for remove in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let mut seed = setup(&mut db);
        let later = confirm_measures(&db, &seed, &[reference(&seed.first.group.measures[0])]);
        select_targets(&mut seed.command, vec![reference(&later.group.measures[0])]);
        let workflow = service(&db, seed.actor.clone());
        let reviewed = workflow
            .prepare("session", db.case, seed.command.clone())
            .unwrap();
        damage_member(&mut db, &seed.first.group.measures[0], remove);
        let before = snapshot(&mut db);

        assert!(workflow
            .prepare("session", db.case, seed.command.clone())
            .is_err());
        assert!(workflow
            .submit("session", db.case, seed.command, confirmation(&reviewed))
            .is_err());

        assert_eq!(snapshot(&mut db), before);
        let count: i64 = db
            .admin
            .query_one(
                "SELECT count(*) FROM case_measure_revisions WHERE owner_operation=$1",
                &[&later.origin.operation_id.as_uuid()],
            )
            .unwrap()
            .get(0);
        assert_eq!(count, 1);
    }
}

#[test]
fn missing_unselected_sibling_invalidates_an_admitted_review_without_partial_commit() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    select_targets(
        &mut seed.command,
        vec![reference(&seed.first.group.measures[0])],
    );
    let reviewed = service(&db, seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let url = db.admin_url.clone();
    let sibling = seed.first.group.measures[1].result.id.as_uuid();
    let admitted = Arc::new(AtomicBool::new(false));
    let observed = admitted.clone();
    let workflow = service_with_format(
        &db,
        seed.actor.clone(),
        FormatCheck(Some(Box::new(move || {
            let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
            let mut tx = client.transaction().unwrap();
            tx.batch_execute("ALTER TABLE case_measure_revisions DISABLE TRIGGER ALL")
                .unwrap();
            assert_eq!(
                tx.execute(
                    "DELETE FROM case_measure_revisions WHERE measure_id=$1 AND revision=1",
                    &[&sibling],
                )
                .unwrap(),
                1
            );
            tx.batch_execute("ALTER TABLE case_measure_revisions ENABLE TRIGGER ALL")
                .unwrap();
            tx.commit().unwrap();
            observed.store(true, Ordering::SeqCst);
        }))),
    );
    let before = snapshot(&mut db);

    assert!(workflow
        .submit("session", db.case, seed.command, confirmation(&reviewed))
        .is_err());

    assert!(admitted.load(Ordering::SeqCst));
    assert_eq!(snapshot(&mut db), before);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM case_measure_revisions WHERE measure_id=$1 AND revision=1",
            &[&seed.first.group.measures[0].result.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}

#[test]
fn original_operation_retry_and_reads_reject_later_corruption_of_selected_source_provenance() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let stored = persist_hearing(&db, seed.actor.clone(), seed.command.clone());
    let query = reads(&db, seed.actor.clone());
    let workflow = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(|| {
            panic!("historical replay cannot repair corrupted sources through fresh admission")
        }))),
    );
    let subject = &seed.first.group.measures[0].result.sources.subject;
    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_subject_revisions DISABLE TRIGGER USER")
        .unwrap();
    assert_eq!(
        tx.execute(
            "UPDATE case_subject_revisions SET changed_by_email='altered-history@example.test'
         WHERE subject_id=$1 AND revision=$2",
            &[&subject.id.as_uuid(), &i64::from(subject.revision.get())],
        )
        .unwrap(),
        1
    );
    tx.batch_execute("ALTER TABLE case_subject_revisions ENABLE TRIGGER USER")
        .unwrap();
    tx.commit().unwrap();
    let before = snapshot(&mut db);

    assert!(workflow
        .prepare("session", db.case, seed.command.clone())
        .is_err());
    assert!(workflow
        .submit(
            "session",
            db.case,
            seed.command.clone(),
            confirmation(&stored.capture.review)
        )
        .is_err());
    assert!(query
        .get("session", db.case, seed.command.hearing_id, None)
        .is_err());
    assert!(query
        .get_operation("session", db.case, seed.command.operation_id)
        .is_err());

    assert_eq!(snapshot(&mut db), before);
}
