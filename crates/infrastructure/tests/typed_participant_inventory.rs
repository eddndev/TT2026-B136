mod case_administration_support;
mod typed_participant_database_support;

use case_administration_support::Fixture;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;
use typed_participant_database_support::Bundle;

fn populated() -> Option<Fixture> {
    let db = Fixture::new()?;
    let mut bundle = Bundle::new(&db);
    let mut client = postgres::Client::connect(&db.admin_url, postgres::NoTls).unwrap();
    let mut tx = client.transaction().unwrap();
    bundle.seed_document(&mut tx, &db);
    bundle.seed_manual(&mut tx, &db);
    bundle.refresh_stamp(&db);
    bundle.insert(&mut tx, &db);
    tx.commit().unwrap();
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    Some(db)
}

#[test]
fn startup_preserves_historical_identity_author_even_after_account_changes() {
    let Some(mut db) = populated() else { return };
    db.admin
        .batch_execute("UPDATE users SET active=FALSE,role='client',email='renamed@example.test'")
        .unwrap();
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let captured: String = db
        .admin
        .query_one("SELECT changed_by_email FROM case_subject_revisions", &[])
        .unwrap()
        .get(0);
    assert_eq!(captured, "owner@example.test");
}

#[test]
fn startup_rejects_altered_provenance_and_projection_after_guards_are_reenabled() {
    for mutation in [
        "ALTER TABLE case_subject_revisions DISABLE TRIGGER typed_immutable;
         UPDATE case_subject_revisions SET changed_at='not a timestamp';
         ALTER TABLE case_subject_revisions ENABLE TRIGGER typed_immutable",
        "ALTER TABLE case_participant_typed_revisions DISABLE TRIGGER typed_immutable;
         UPDATE case_participant_typed_revisions SET changed_by_email=' owner@example.test';
         ALTER TABLE case_participant_typed_revisions ENABLE TRIGGER typed_immutable",
        "ALTER TABLE case_subject_revisions DROP CONSTRAINT subject_projection;
         ALTER TABLE case_subject_revisions ADD CONSTRAINT subject_projection CHECK(TRUE);
         ALTER TABLE case_subject_revisions DISABLE TRIGGER typed_immutable;
         UPDATE case_subject_revisions SET display_name='Different declared name';
         ALTER TABLE case_subject_revisions ENABLE TRIGGER typed_immutable",
        "ALTER TABLE case_participant_typed_revisions DROP CONSTRAINT typed_projection;
         ALTER TABLE case_participant_typed_revisions ADD CONSTRAINT typed_projection CHECK(TRUE);
         ALTER TABLE case_participant_typed_revisions DISABLE TRIGGER typed_immutable;
         UPDATE case_participant_typed_revisions SET role_kind='other';
         ALTER TABLE case_participant_typed_revisions ENABLE TRIGGER typed_immutable",
    ] {
        let Some(mut db) = populated() else { return };
        db.admin.batch_execute(mutation).unwrap();
        assert!(
            PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn startup_detects_orphaned_review_and_missing_union_revisions_after_administrative_tampering() {
    for mutation in [
        "SET session_replication_role=replica;
         INSERT INTO participant_identity_reviews SELECT 'ffffffff-ffff-ffff-ffff-ffffffffffff',revision,
            review_canonical,review_digest,submission_canonical,submission_digest FROM participant_identity_reviews;
         SET session_replication_role=origin",
        "SET session_replication_role=replica; DELETE FROM case_participant_revisions; SET session_replication_role=origin",
        "SET session_replication_role=replica; DELETE FROM subject_identity_reviews; SET session_replication_role=origin",
    ] {
        let Some(mut db)=populated() else{return};
        db.admin.batch_execute(mutation).unwrap();
        assert!(PostgresCaseRepository::open(&db.runtime_url,Arc::new(RingSha256Hasher)).is_err(),"{mutation}");
    }
}

#[test]
fn manual_provenance_is_checked_beyond_the_first_page() {
    let Some(mut db) = Fixture::new() else { return };
    let mut client = postgres::Client::connect(&db.admin_url, postgres::NoTls).unwrap();
    let mut tx = client.transaction().unwrap();
    for i in 1..=65_u128 {
        let id = uuid::Uuid::from_u128(i);
        tx.execute(
            "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
            &[&id, &db.case.as_uuid()],
        )
        .unwrap();
        tx.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,
            directory_status,values_digest,changed_at,changed_by,changed_by_email)
            VALUES($1,1,'Name','Role','active',sha256(participant_values_bytes('Name','Role',NULL,NULL,'active')),
                '2025-01-01T00:00:00Z',$2,'owner@example.test')",&[&id,&db.owner.as_uuid()]).unwrap();
    }
    tx.commit().unwrap();
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    db.admin.batch_execute("ALTER TABLE case_participant_revisions DISABLE TRIGGER participant_revision_immutable;
        UPDATE case_participant_revisions SET changed_at='invalid' WHERE participant_id='00000000-0000-0000-0000-000000000041';
        ALTER TABLE case_participant_revisions ENABLE TRIGGER participant_revision_immutable").unwrap();
    assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err());
}
