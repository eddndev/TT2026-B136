use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn membership_revoked_after_admission_prevents_every_group_write() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let user = db.user("litigator", true);
    let actor = principal(&mut db, user);
    let draft = service(&db, actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let url = db.admin_url.clone();
    let case = db.case;
    let admitted = Arc::new(AtomicBool::new(false));
    let observed = admitted.clone();
    let workflow = service_with_format(
        &db,
        actor,
        FormatCheck(Some(Box::new(move || {
            let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
            assert_eq!(
                client
                    .execute(
                        "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                        &[&case.as_uuid(), &user.as_uuid()],
                    )
                    .unwrap(),
                1
            );
            observed.store(true, Ordering::SeqCst);
        }))),
    );
    let before = snapshot(&mut db);

    let result = workflow.submit("session", db.case, seed.command, confirmation(&draft));

    assert!(admitted.load(Ordering::SeqCst));
    assert!(result.is_err());
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                &[&case.as_uuid(), &user.as_uuid()],
            )
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}

#[test]
fn selected_subject_provenance_corrupted_after_admission_prevents_every_group_write() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let draft = service(&db, seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    let url = db.admin_url.clone();
    let subject = seed.subject.id;
    let revision = i64::from(seed.subject.revision.get());
    let admitted = Arc::new(AtomicBool::new(false));
    let observed = admitted.clone();
    let workflow = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(move || {
            let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
            let mut tx = client.transaction().unwrap();
            tx.batch_execute("ALTER TABLE case_subject_revisions DISABLE TRIGGER USER")
                .unwrap();
            assert_eq!(
                tx.execute(
                    "UPDATE case_subject_revisions SET changed_by_email='altered-source@example.test'
                     WHERE subject_id=$1 AND revision=$2",
                    &[&subject.as_uuid(), &revision],
                )
                .unwrap(),
                1
            );
            tx.batch_execute("ALTER TABLE case_subject_revisions ENABLE TRIGGER USER")
                .unwrap();
            tx.commit().unwrap();
            observed.store(true, Ordering::SeqCst);
        }))),
    );
    let before = snapshot(&mut db);

    let result = workflow.submit("session", db.case, seed.command, confirmation(&draft));

    assert!(admitted.load(Ordering::SeqCst));
    assert!(result.is_err());
    assert_eq!(snapshot(&mut db), before);
    let row = db.admin.query_one(
        "SELECT changed_by_email,values_digest FROM case_subject_revisions WHERE subject_id=$1 AND revision=$2",
        &[&subject.as_uuid(), &revision],
    ).unwrap();
    assert_eq!(row.get::<_, String>(0), "altered-source@example.test");
    assert_eq!(
        row.get::<_, Vec<u8>>(1).as_slice(),
        seed.subject.values_digest.as_bytes().as_slice()
    );
}
