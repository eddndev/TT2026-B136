use super::*;

#[test]
fn changed_retained_subject_provenance_rejects_the_original_administrative_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    reopened(&db, &seed.actor, &original);
    let storage = store(&db);
    let workflow = existing_service(&db, seed.actor.clone(), storage.clone());
    let id = seed.subject.id.as_uuid();
    let revision = seed.subject.revision.get();

    damage(
        &mut db,
        &["case_subject_revisions"],
        &format!(
            "UPDATE case_subject_revisions SET changed_by_email='altered-source@example.test'
         WHERE subject_id='{id}' AND revision={revision}",
        ),
    );

    let digest: Vec<u8> = db
        .admin
        .query_one(
            "SELECT values_digest FROM case_subject_revisions WHERE subject_id=$1 AND revision=$2",
            &[&id, &i64::from(revision)],
        )
        .unwrap()
        .get(0);
    assert_eq!(digest.as_slice(), seed.subject.values_digest.as_bytes());
    reject_original(&mut db, &storage, &workflow, &seed.actor, &original);
}

#[test]
fn changed_retained_support_name_rejects_the_original_administrative_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    reopened(&db, &seed.actor, &original);
    let storage = store(&db);
    let workflow = existing_service(&db, seed.actor.clone(), storage.clone());
    let id = seed.record.id.as_uuid();
    let version = seed.record.version.get();

    damage(
        &mut db,
        &["documents"],
        &format!(
            "UPDATE documents SET name='altered-retained-support.pdf'
         WHERE id='{id}' AND version={version}",
        ),
    );

    let digest: Vec<u8> = db
        .admin
        .query_one(
            "SELECT digest FROM documents WHERE id=$1 AND version=$2",
            &[&id, &i64::from(version)],
        )
        .unwrap()
        .get(0);
    assert_eq!(digest.as_slice(), seed.record.digest.as_bytes());
    assert_ne!(
        original.capture.review.support.name,
        "altered-retained-support.pdf"
    );
    reject_original(&mut db, &storage, &workflow, &seed.actor, &original);
}
