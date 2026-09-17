#[allow(dead_code)]
mod legacy_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/deadline_profile_catalog_support/values.rs"]
mod profile_values;

use application::deadline_profiles::*;
use domain::{crypto::DocumentHasher, identity::UserId};
use infrastructure::RingSha256Hasher;
use legacy_database_support::{Database, Source};
use serde_json::Value;
use uuid::Uuid;

#[test]
fn initial_import_rejects_a_complete_unaudited_profile_with_its_source_event() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    source.inspect().check_target(&db.url).unwrap();
    seed_profile(&mut db);
    let counts = db.client.query_one("SELECT (SELECT count(*) FROM deadline_profiles),
        (SELECT count(*) FROM deadline_profile_revisions),(SELECT count(*) FROM deadline_source_events)", &[]).unwrap();
    assert_eq!(
        (
            counts.get::<_, i64>(0),
            counts.get::<_, i64>(1),
            counts.get::<_, i64>(2)
        ),
        (1, 1, 1)
    );
    assert_rejected_without_changes(&mut db, &source);
}

#[test]
fn initial_import_rejects_each_isolated_profile_or_event_table_from_a_partial_restore() {
    for retained in [
        "deadline_profiles",
        "deadline_profile_revisions",
        "deadline_source_events",
    ] {
        let Some(mut db) = Database::new() else {
            return;
        };
        let source = Source::new();
        db.seed_case(source.case_id);
        seed_profile(&mut db);
        // Simulate partial administrative restoration; all guards are restored before inspection.
        for table in [
            "deadline_profiles",
            "deadline_profile_revisions",
            "deadline_source_events",
        ] {
            db.client
                .batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
                .unwrap();
        }
        for table in [
            "deadline_profiles",
            "deadline_profile_revisions",
            "deadline_source_events",
        ] {
            if table != retained {
                assert_eq!(
                    db.client
                        .execute(&format!("DELETE FROM {table}"), &[])
                        .unwrap(),
                    1
                );
            }
        }
        for table in [
            "deadline_profiles",
            "deadline_profile_revisions",
            "deadline_source_events",
        ] {
            db.client
                .batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
                .unwrap();
        }
        assert_eq!(
            db.client
                .query_one(&format!("SELECT count(*) FROM {retained}"), &[])
                .unwrap()
                .get::<_, i64>(0),
            1
        );
        assert_rejected_without_changes(&mut db, &source);
    }
}

fn seed_profile(db: &mut Database) {
    let actor = db
        .client
        .query_one("SELECT id,email FROM users LIMIT 1", &[])
        .unwrap();
    let actor_id: Uuid = actor.get(0);
    let email: String = actor.get(1);
    let definition = profile_values::definition(None);
    let bytes = deadline_profile_definition_bytes(&definition);
    let digest = RingSha256Hasher.hash_bytes(&bytes);
    let command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: DeadlineProfileId::new(),
        change: DeadlineProfileChange::Publish { definition },
    };
    let receipt = deadline_profile_submission_bytes(
        UserId::from_uuid(actor_id),
        &command,
        DeadlineProfileAlgorithm::V1,
        digest,
    );
    let mut tx = db.client.transaction().unwrap();
    tx.execute(
        "INSERT INTO deadline_profiles(id) VALUES($1)",
        &[&command.profile_id.as_uuid()],
    )
    .unwrap();
    tx.execute("INSERT INTO deadline_profile_revisions(profile_id,revision,definition_canonical,definition_digest,algorithm,
        operation_id,action,submission_canonical,submission_digest,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,1,$2,sha256($2),1,$3,'publish',$4,sha256($4),0,0,$5,$6)",
        &[&command.profile_id.as_uuid(), &bytes, &command.operation_id.as_uuid(), &receipt, &actor_id, &email]).unwrap();
    tx.commit().unwrap();
}

fn snapshot(db: &mut Database) -> Value {
    db.client.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM deadline_profiles p),
        'revisions',(SELECT jsonb_agg(to_jsonb(p) ORDER BY profile_id,revision) FROM deadline_profile_revisions p),
        'events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY sequence) FROM deadline_source_events e),
        'sequence',(SELECT jsonb_build_object('last_value',last_value,'is_called',is_called) FROM deadline_source_events_sequence))", &[]).unwrap().get(0)
}
fn assert_rejected_without_changes(db: &mut Database, source: &Source) {
    assert_eq!(db.counts(), (0, 0, 0));
    let before = db.stored_state();
    let profiles = snapshot(db);
    let document = std::fs::read(source.document_path()).unwrap();
    let audit = std::fs::read(source.audit_path()).unwrap();
    assert!(
        source.inspect().check_target(&db.url).is_err(),
        "occupied deadline data must reject initial import inspection"
    );
    assert!(
        source.inspect().apply(&db.url).is_err(),
        "occupied deadline data must reject initial import application"
    );
    assert_eq!(db.stored_state(), before);
    assert_eq!(snapshot(db), profiles);
    assert_eq!(std::fs::read(source.document_path()).unwrap(), document);
    assert_eq!(std::fs::read(source.audit_path()).unwrap(), audit);
    source.assert_unmarked();
}
