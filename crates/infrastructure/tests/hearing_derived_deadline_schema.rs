use crate::case_administration_support::Fixture;
use postgres::error::SqlState;

const TABLE: &str = "case_hearing_derived_deadline_origins";

#[test]
fn migration_preserves_existing_state_and_installs_empty_immutable_origins() {
    let Some(mut db) = Fixture::new() else { return };
    let exists: Option<String> = db
        .admin
        .query_one("SELECT to_regclass($1)::text", &[&TABLE])
        .unwrap()
        .get(0);
    assert!(
        exists.is_some(),
        "missing hearing-derived deadline origin table"
    );
    let before = db.snapshot();
    let count: i64 = db
        .admin
        .query_one(&format!("SELECT count(*) FROM {TABLE}"), &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    db.migrate();
    assert_eq!(before, db.snapshot());
    for command in [
        format!("UPDATE {TABLE} SET case_id=case_id"),
        format!("DELETE FROM {TABLE}"),
        format!("TRUNCATE {TABLE}"),
    ] {
        assert_eq!(
            db.runtime().batch_execute(&command).unwrap_err().code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE)
        );
        assert_eq!(
            db.admin.batch_execute(&command).unwrap_err().code(),
            Some(&SqlState::CHECK_VIOLATION)
        );
    }
    open(&db).unwrap();
}

fn open(
    db: &Fixture,
) -> Result<infrastructure::PostgresCaseRepository, application::ApplicationError> {
    infrastructure::PostgresCaseRepository::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
    )
}

#[test]
fn startup_requires_an_origin_for_each_compound_audit_marker() {
    use domain::audit::AuditLog;
    let Some(db) = Fixture::new() else { return };
    let mut audit = infrastructure::PostgresAuditLog::open(&db.runtime_url).unwrap();
    audit.append("owner@example.test","hearing_derived_deadline.registered","hrdc1:operation:00000000-0000-0000-0000-000000000001:capture:0000000000000000000000000000000000000000000000000000000000000000",db.at).unwrap();
    assert!(open(&db).is_err(), "accepted compound audit with no origin");
}

#[test]
fn startup_checks_stored_origin_canonical_evidence_and_original_actor_role() {
    use crate::hearing_derived_deadline_storage_support as origin;
    let Some(mut db) = Fixture::new() else { return };
    let record = origin::record(&mut db);
    origin::insert(&mut db, &record);
    open(&db).unwrap();
    db.admin.batch_execute("ALTER TABLE case_hearing_derived_deadline_origins DISABLE TRIGGER USER; UPDATE case_hearing_derived_deadline_origins SET actor_role='litigator'; ALTER TABLE case_hearing_derived_deadline_origins ENABLE TRIGGER USER;").unwrap();
    assert!(
        open(&db).is_err(),
        "accepted substituted original actor role"
    );
    db.admin.batch_execute("ALTER TABLE case_hearing_derived_deadline_origins DISABLE TRIGGER USER; UPDATE case_hearing_derived_deadline_origins SET actor_role='owner'; ALTER TABLE case_hearing_derived_deadline_origins ENABLE TRIGGER USER;").unwrap();
    open(&db).unwrap();
}

#[test]
fn startup_rejects_rehashed_canonical_corruption_and_substituted_links() {
    use crate::hearing_derived_deadline_storage_support as origin;
    let Some(mut db) = Fixture::new() else { return };
    let record = origin::record(&mut db);
    origin::insert(&mut db, &record);
    let stored = db
        .admin
        .query_one(
            "SELECT review_canonical,capture_canonical,
        source_event_sequence,audit_sequence FROM case_hearing_derived_deadline_origins",
            &[],
        )
        .unwrap();
    let review: Vec<u8> = stored.get(0);
    let capture: Vec<u8> = stored.get(1);
    let event: i64 = stored.get(2);
    let audit: i64 = stored.get(3);
    for mutation in [
        "review_canonical=set_byte(review_canonical,250,get_byte(review_canonical,250)#1),review_digest=sha256(set_byte(review_canonical,250,get_byte(review_canonical,250)#1))",
        "capture_canonical=set_byte(capture_canonical,200,get_byte(capture_canonical,200)#1),capture_digest=sha256(set_byte(capture_canonical,200,get_byte(capture_canonical,200)#1))",
        "source_event_sequence=(SELECT min(sequence) FROM deadline_source_events WHERE source_kind='profile')",
        "audit_sequence=0",
    ] {
        db.admin.batch_execute(&format!("ALTER TABLE {TABLE} DISABLE TRIGGER USER;
            UPDATE {TABLE} SET {mutation}; ALTER TABLE {TABLE} ENABLE TRIGGER USER")).unwrap();
        assert!(open(&db).is_err(), "accepted corruption: {mutation}");
        db.admin.batch_execute(&format!("ALTER TABLE {TABLE} DISABLE TRIGGER USER")).unwrap();
        db.admin.execute(&format!("UPDATE {TABLE} SET review_canonical=$1,review_digest=sha256($1),
            capture_canonical=$2,capture_digest=sha256($2),source_event_sequence=$3,audit_sequence=$4"),
            &[&review,&capture,&event,&audit]).unwrap();
        db.admin.batch_execute(&format!("ALTER TABLE {TABLE} ENABLE TRIGGER USER")).unwrap();
        open(&db).unwrap();
    }
    let chain: Vec<u8> = db
        .admin
        .query_one(
            "SELECT chain FROM audit_events WHERE sequence=$1",
            &[&audit],
        )
        .unwrap()
        .get(0);
    db.admin
        .execute(
            "UPDATE audit_events SET chain=decode(repeat('00',32),'hex') WHERE sequence=$1",
            &[&audit],
        )
        .unwrap();
    assert!(
        open(&db).is_err(),
        "accepted altered compound audit commitment"
    );
    db.admin
        .execute(
            "UPDATE audit_events SET chain=$2 WHERE sequence=$1",
            &[&audit, &chain],
        )
        .unwrap();
    open(&db).unwrap();
}
