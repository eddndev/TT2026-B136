use crate::audit_query_support::*;
use application::audit_context::AuditScope;
use domain::audit::{AuditContext, AuditEvent, AuditSource, ChainedEvent, GENESIS_PREVIOUS};
use domain::crypto::DocumentHasher;
use postgres::error::SqlState;
use uuid::Uuid;

fn http_context(db: &Fixture) -> AuditContext {
    AuditContext {
        user_id: Some(db.owner),
        ip_address: Some("2001:db8::42".parse().unwrap()),
        source: AuditSource::Http,
        request_id: Some(Uuid::parse_str("10000000-0000-4000-8000-000000000042").unwrap()),
    }
}

fn legacy_entry(db: &mut Fixture) -> (ChainedEvent, Vec<u8>) {
    let event = AuditEvent::new(0, db.at, "legacy@example.test", "legacy.read", "case-old");
    let text = "2025-01-01T00:00:00.123456789Z";
    let mut canonical = 0u64.to_be_bytes().to_vec();
    for field in [text, "legacy@example.test", "legacy.read", "case-old"] {
        canonical.extend_from_slice(&(field.len() as u32).to_be_bytes());
        canonical.extend_from_slice(field.as_bytes());
    }
    let mut linked = GENESIS_PREVIOUS.as_bytes().to_vec();
    linked.extend_from_slice(&canonical);
    let chain = RingSha256Hasher.hash_bytes(&linked);
    db.admin
        .execute(
            "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
             VALUES(0,$1,$2,$3,$4,$5)",
            &[
                &text,
                &event.actor,
                &event.action,
                &event.resource,
                &&chain.as_bytes()[..],
            ],
        )
        .unwrap();
    (ChainedEvent { event, chain }, canonical)
}

fn stored_context(db: &mut Fixture, sequence: u64) -> Option<String> {
    db.admin
        .query_one(
            "SELECT context_json FROM audit_events WHERE sequence=$1",
            &[&(sequence as i64)],
        )
        .unwrap()
        .get(0)
}

fn selected_request(db: &Fixture) -> AuditEventQuery {
    AuditEventQuery::new(
        db.at - Duration::seconds(1),
        db.at + Duration::seconds(1),
        None,
        Some("context.http"),
        None,
        20,
        None,
    )
    .unwrap()
}

fn complete_rows(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY sequence),'[]'::jsonb)
             FROM audit_events a",
            &[],
        )
        .unwrap()
        .get(0)
}

#[test]
fn migration_keeps_legacy_context_null_and_original_bytes_hash_and_columns() {
    let Some(mut db) = Fixture::old() else { return };
    let (expected, canonical) = legacy_entry(&mut db);
    let before = original_columns(&mut db);
    db.migrate();
    assert_eq!(original_columns(&mut db), before);
    assert_eq!(stored_context(&mut db, 0), None);
    let events = trail(&db);
    assert_eq!(events, vec![expected]);
    assert_eq!(events[0].event.context, None);
    assert_eq!(events[0].event.canonical_bytes().unwrap(), canonical);
    db.migrate();
    assert_eq!(original_columns(&mut db), before);
    assert_eq!(stored_context(&mut db, 0), None);
    assert_eq!(trail(&db), events);
    assert_chain(&db);
}

#[test]
fn http_context_round_trips_through_append_load_and_authorized_query() {
    let Some(mut db) = Fixture::new() else { return };
    let expected = http_context(&db);
    let scope = AuditScope::new(expected.clone());
    let sequence = scope.run(|| append(&db, "owner@example.test", "context.http", "case", db.at));
    assert_eq!(scope.snapshot().unwrap(), expected);
    let stored = stored_context(&mut db, sequence).unwrap();
    assert!(stored.len() <= 1024);
    assert_eq!(
        serde_json::from_str::<AuditContext>(&stored).unwrap(),
        expected
    );
    let loaded = trail(&db);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].event.context, Some(expected.clone()));
    let page = scope.run(|| {
        store(&db)
            .read(&owner(&db), &selected_request(&db), db.at)
            .unwrap()
    });
    assert_eq!(page.events, vec![loaded[0].event.clone()]);
    let read = trail(&db).pop().unwrap();
    assert_eq!(read.event.action, "audit.events_read");
    assert_eq!(read.event.context, Some(expected));
    assert_chain(&db);
}

#[test]
fn administrator_context_tampering_breaks_the_existing_chain() {
    let Some(mut db) = Fixture::new() else { return };
    let expected = http_context(&db);
    let sequence = AuditScope::new(expected.clone())
        .run(|| append(&db, "owner@example.test", "context.http", "case", db.at));
    let original = trail(&db);
    let mut changed = expected;
    changed.ip_address = Some("192.0.2.89".parse().unwrap());
    db.admin
        .execute(
            "UPDATE audit_events SET context_json=$1 WHERE sequence=$2",
            &[
                &serde_json::to_string(&changed).unwrap(),
                &(sequence as i64),
            ],
        )
        .unwrap();
    let corrupted = trail(&db);
    assert_eq!(corrupted[0].chain, original[0].chain);
    assert_eq!(corrupted[0].event.context, Some(changed));
    assert_eq!(
        domain::audit::verify_chain(&RingSha256Hasher, &corrupted).unwrap(),
        ChainVerification::Broken {
            first_broken_index: 0
        }
    );
}

#[test]
fn internal_append_does_not_infer_a_user_uuid_or_network_origin_from_actor_text() {
    let Some(mut db) = Fixture::new() else { return };
    let sequence = append(&db, &db.owner.to_string(), "internal.work", "case", db.at);
    let expected = AuditContext {
        user_id: None,
        ip_address: None,
        source: AuditSource::Internal,
        request_id: None,
    };
    assert_eq!(trail(&db)[0].event.context, Some(expected.clone()));
    let stored = stored_context(&mut db, sequence).unwrap();
    assert_eq!(
        serde_json::from_str::<AuditContext>(&stored).unwrap(),
        expected
    );
    assert_chain(&db);
}

#[test]
fn runtime_role_cannot_rewrite_or_remove_persisted_context() {
    let Some(mut db) = Fixture::new() else { return };
    AuditScope::new(http_context(&db))
        .run(|| append(&db, "owner@example.test", "context.http", "case", db.at));
    let before = complete_rows(&mut db);
    let mut runtime = db.runtime();
    for statement in [
        "UPDATE audit_events SET context_json=NULL",
        "UPDATE audit_events SET context_json='{}'",
        "ALTER TABLE audit_events DROP COLUMN context_json",
    ] {
        assert_eq!(
            runtime.batch_execute(statement).unwrap_err().code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    assert_eq!(complete_rows(&mut db), before);
    assert_chain(&db);
}

#[test]
fn context_storage_rejects_more_than_1024_bytes_without_changing_history() {
    let Some(mut db) = Fixture::new() else { return };
    let sequence = AuditScope::new(http_context(&db))
        .run(|| append(&db, "owner@example.test", "context.http", "case", db.at));
    let before = complete_rows(&mut db);
    let error = db
        .admin
        .execute(
            "UPDATE audit_events SET context_json=$1 WHERE sequence=$2",
            &[&"x".repeat(1025), &(sequence as i64)],
        )
        .unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(complete_rows(&mut db), before);
    assert_chain(&db);
}

#[test]
fn dump_restore_preserves_legacy_and_http_context_before_appending_a_successor() {
    let Some(mut db) = Fixture::new() else { return };
    legacy_entry(&mut db);
    let scope = AuditScope::new(http_context(&db));
    scope.run(|| append(&db, "owner@example.test", "context.http", "case", db.at));
    let request = selected_request(&db);
    let page = scope.run(|| store(&db).read(&owner(&db), &request, db.at).unwrap());
    let before = complete_rows(&mut db);
    let expected = trail(&db);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("audit-context.dump");
    let result = std::process::Command::new("pg_dump")
        .arg("--dbname")
        .arg(&db.admin_url)
        .arg("--schema")
        .arg(&db.schema)
        .arg("--format=custom")
        .arg("--file")
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "isolated audit context dump failed"
    );
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let result = std::process::Command::new("pg_restore")
        .arg("--exit-on-error")
        .arg("--dbname")
        .arg(&db.admin_url)
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "isolated audit context restore failed"
    );
    assert_eq!(complete_rows(&mut db), before);
    db.migrate();
    assert_eq!(complete_rows(&mut db), before);
    assert_eq!(trail(&db), expected);
    assert_eq!(stored_context(&mut db, 0), None);
    let restored = scope.run(|| store(&db).read(&owner(&db), &request, db.at).unwrap());
    assert_eq!(restored.events, page.events);
    let sequence = append(&db, "worker", "after.restore", "case", db.at);
    assert_eq!(sequence, expected.len() as u64 + 1);
    assert_eq!(
        trail(&db)
            .last()
            .unwrap()
            .event
            .context
            .as_ref()
            .unwrap()
            .source,
        AuditSource::Internal
    );
    assert_chain(&db);
}
