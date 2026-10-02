use crate::audit_query_support::*;
use postgres::error::SqlState;

#[test]
fn populated_history_migration_preserves_canonical_text_chain_and_nanosecond_keys() {
    let Some(mut db) = Fixture::old() else { return };
    let times = [
        "0000-02-29T00:00:00Z",
        "1969-12-31T23:59:59.999999999Z",
        "2025-01-01T00:00:00Z",
        "2025-01-01T00:00:00.000000001Z",
        "2025-01-01T00:00:00.1Z",
        "9999-12-31T23:59:59.999999999Z",
    ];
    let mut previous = domain::audit::GENESIS_PREVIOUS;
    for (i, text) in times.into_iter().enumerate() {
        let time =
            OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339).unwrap();
        let event = domain::audit::AuditEvent::new(
            i as u64,
            time,
            "old actor",
            "old.action",
            "old resource",
        );
        let chain = domain::audit::chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
        db.admin
            .execute(
                "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
            VALUES($1,$2,$3,$4,$5,$6)",
                &[
                    &(i as i64),
                    &text,
                    &event.actor,
                    &event.action,
                    &event.resource,
                    &&chain.as_bytes()[..],
                ],
            )
            .unwrap();
        previous = chain;
    }
    let before = original_columns(&mut db);
    db.migrate();
    assert_eq!(original_columns(&mut db), before);
    for row in db.admin.query("SELECT timestamp,timestamp_seconds,timestamp_nanos FROM audit_events ORDER BY sequence", &[]).unwrap() {
        let at = OffsetDateTime::parse(row.get::<_, &str>(0), &time::format_description::well_known::Rfc3339).unwrap();
        assert_eq!(row.get::<_, i64>(1), at.unix_timestamp());
        assert_eq!(row.get::<_, i32>(2), at.nanosecond() as i32);
    }
    db.migrate();
    assert_eq!(original_columns(&mut db), before);
    assert_chain(&db);
}

#[test]
fn malformed_historical_timestamp_aborts_additive_migration_without_rewriting_history() {
    let Some(mut db) = Fixture::old() else { return };
    db.admin
        .batch_execute(
            "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
        VALUES(0,'2025-02-30T00:00:00Z','a','b','c',decode(repeat('00',32),'hex'))",
        )
        .unwrap();
    let before = original_columns(&mut db);
    assert!(infrastructure::initialize_database(&db.admin_url, &db.role).is_err());
    assert_eq!(original_columns(&mut db), before);
    let projection: i64 = db.admin.query_one("SELECT count(*) FROM pg_attribute
        WHERE attrelid='audit_events'::regclass AND attname='timestamp_seconds' AND NOT attisdropped", &[]).unwrap().get(0);
    assert_eq!(projection, 0);
}

#[test]
fn runtime_projection_is_generated_and_audit_history_remains_append_only() {
    let Some(db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    let mut runtime = db.runtime();
    for sql in [
        "UPDATE audit_events SET actor='changed'",
        "DELETE FROM audit_events",
        "TRUNCATE audit_events",
        "ALTER TABLE audit_events DROP COLUMN timestamp_nanos",
    ] {
        assert_eq!(
            runtime.batch_execute(sql).unwrap_err().code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    let error = runtime.batch_execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain,timestamp_seconds)
        VALUES(1,'2025-01-01T00:00:00Z','a','b','c',decode(repeat('00',32),'hex'),0)").unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::GENERATED_ALWAYS));
    for invalid in [
        "2025-01-01T00:00:00.10Z",
        "2025-01-01T00:00:00+00:00",
        "2025-01-01T24:00:00Z",
    ] {
        assert!(runtime
            .query_one("SELECT audit_timestamp_parts($1)", &[&invalid])
            .is_err());
    }
    assert_chain(&db);
}

#[test]
fn startup_rejects_index_function_rls_expression_and_privilege_damage() {
    let Some(mut db) = Fixture::new() else { return };
    for (damage, repair) in [
        ("DROP INDEX audit_events_chronological".to_owned(), "CREATE INDEX audit_events_chronological ON audit_events(timestamp_seconds,timestamp_nanos,sequence)".to_owned()),
        ("ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY".to_owned(), "ALTER TABLE audit_events DISABLE ROW LEVEL SECURITY".to_owned()),
        ("ALTER FUNCTION audit_timestamp_parts(text) VOLATILE".to_owned(), "ALTER FUNCTION audit_timestamp_parts(text) IMMUTABLE".to_owned()),
        ("GRANT SELECT ON audit_events TO PUBLIC".to_owned(), "REVOKE SELECT ON audit_events FROM PUBLIC".to_owned()),
        (format!("GRANT UPDATE ON audit_events TO {}", db.role), format!("REVOKE UPDATE ON audit_events FROM {}", db.role)),
        (format!("GRANT EXECUTE ON FUNCTION audit_timestamp_parts(text) TO {} WITH GRANT OPTION", db.role), format!("REVOKE GRANT OPTION FOR EXECUTE ON FUNCTION audit_timestamp_parts(text) FROM {}", db.role)),
    ] {
        db.admin.batch_execute(&damage).unwrap();
        assert!(matches!(PostgresAuditEventStore::open(&db.runtime_url), Err(ApplicationError::InvalidConfiguration(_))), "{damage}");
        db.admin.batch_execute(&repair).unwrap();
        store(&db);
    }
    db.admin
        .batch_execute("ALTER TABLE audit_events ALTER COLUMN timestamp_nanos DROP EXPRESSION")
        .unwrap();
    assert!(matches!(
        PostgresAuditEventStore::open(&db.runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn chronological_projection_has_an_usable_btree_seek_plan() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    db.admin.batch_execute("SET enable_seqscan=off").unwrap();
    let rows = db
        .admin
        .query(
            "EXPLAIN (COSTS OFF) SELECT sequence FROM audit_events
        WHERE (timestamp_seconds,timestamp_nanos,sequence)>(0,0,0)
        ORDER BY timestamp_seconds,timestamp_nanos,sequence LIMIT 2",
            &[],
        )
        .unwrap();
    let plan = rows
        .iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plan.contains("audit_events_chronological"), "{plan}");
}

#[test]
fn restored_function_body_does_not_hide_previously_corrupted_generated_values() {
    let Some(mut db) = Fixture::new() else { return };
    let definition: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef('audit_timestamp_parts(text)'::regprocedure)",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin
        .batch_execute(
            "CREATE OR REPLACE FUNCTION audit_timestamp_parts(value TEXT) RETURNS BIGINT[]
        LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE SET search_path=pg_catalog AS $$
        BEGIN RETURN ARRAY[0::bigint,0::bigint]; END $$",
        )
        .unwrap();
    assert!(matches!(
        PostgresAuditEventStore::open(&db.runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    db.admin
        .batch_execute(
            "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
        VALUES(0,'2025-01-01T00:00:00Z','a','b','c',decode(repeat('00',32),'hex'))",
        )
        .unwrap();
    db.admin.batch_execute(&definition).unwrap();
    assert!(matches!(
        PostgresAuditEventStore::open(&db.runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}
