use crate::audit_query_support::*;

const LOOKUPS: [&str; 4] = [
    "audit_events_action_resource",
    "audit_events_chronological",
    "audit_events_pkey",
    "audit_events_resource",
];

fn indexes(db: &mut Fixture) -> Vec<String> {
    db.admin
        .query(
            "SELECT c.relname::text FROM pg_index i JOIN pg_class c ON c.oid=i.indexrelid
        WHERE i.indrelid='audit_events'::regclass ORDER BY 1",
            &[],
        )
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect()
}

fn rejected(db: &Fixture) -> bool {
    matches!(
        PostgresAuditEventStore::open(&db.runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    )
}

#[test]
fn migration_adds_exact_lookup_indexes_and_repeating_it_changes_nothing() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    assert_eq!(indexes(&mut db), LOOKUPS);
    let before = original_columns(&mut db);
    db.migrate();
    assert_eq!(indexes(&mut db), LOOKUPS);
    assert_eq!(original_columns(&mut db), before);
    store(&db);
    assert_chain(&db);
}

#[test]
fn schema_without_lookup_indexes_is_rejected_until_migrated() {
    let Some(mut db) = Fixture::new() else { return };
    append(&db, "actor", "op", "r", db.at);
    let before = original_columns(&mut db);
    db.admin
        .batch_execute("DROP INDEX audit_events_action_resource, audit_events_resource")
        .unwrap();
    assert!(rejected(&db));
    db.migrate();
    assert_eq!(indexes(&mut db), LOOKUPS);
    assert_eq!(original_columns(&mut db), before);
    store(&db);
    assert_chain(&db);
}

#[test]
fn startup_rejects_altered_missing_or_additional_lookup_indexes() {
    let Some(mut db) = Fixture::new() else { return };
    let canonical = "DROP INDEX IF EXISTS audit_events_action_resource, audit_events_resource;
        CREATE INDEX audit_events_action_resource ON audit_events(action,resource);
        CREATE INDEX audit_events_resource ON audit_events(resource)";
    for damage in [
        "DROP INDEX audit_events_action_resource",
        "DROP INDEX audit_events_resource",
        "DROP INDEX audit_events_action_resource;
        CREATE INDEX audit_events_action_resource ON audit_events(resource,action)",
        "DROP INDEX audit_events_action_resource;
        CREATE INDEX audit_events_action_resource ON audit_events(action)",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events(resource DESC)",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events(resource text_pattern_ops)",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events(resource COLLATE \"C\")",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events(resource) WHERE action<>''",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events USING hash(resource)",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events(resource) INCLUDE (action)",
        "DROP INDEX audit_events_resource;
        CREATE INDEX audit_events_resource ON audit_events(lower(resource))",
        "CREATE INDEX audit_events_actor ON audit_events(actor)",
    ] {
        db.admin.batch_execute(damage).unwrap();
        assert!(rejected(&db), "{damage}");
        db.admin
            .batch_execute("DROP INDEX IF EXISTS audit_events_actor")
            .unwrap();
        db.admin.batch_execute(canonical).unwrap();
        assert_eq!(indexes(&mut db), LOOKUPS);
        store(&db);
    }
}

fn plan(db: &mut Fixture, sql: &str) -> String {
    db.admin
        .query(&format!("EXPLAIN (COSTS OFF) {sql}"), &[])
        .unwrap()
        .iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn coverage_lookups_over_scanner_dominated_history_use_lookup_indexes() {
    let Some(mut db) = Fixture::new() else { return };
    // Mirrors a history dominated by repeated scanner events: two actions and
    // a few dozen resources, with sparse functional markers.
    db.admin
        .batch_execute(
            "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
        SELECT n,'2025-01-01T00:00:00Z','system:activity-alerts',
            CASE WHEN n%26=0 THEN 'alert.scan_completed' ELSE 'alert.reconciled' END,
            CASE WHEN n%26=0 THEN 'alert-scan'
            ELSE 'subject:'||(n%4)||':'||md5((n%26)::text)||':scheduled:0:superseded:0' END,
            decode(repeat('00',32),'hex')
        FROM generate_series(0,59999) n;
        INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
        SELECT 60000+n,'2025-01-01T00:00:00Z','owner@example.test',
            CASE WHEN n%2=0 THEN 'precautionary_hearing.schedule'
            ELSE 'resource_hearing.registered' END,
            CASE WHEN n%2=0 THEN 'ph1:case:'||md5(n::text)||':hearing:'||md5((n+1)::text)
                ||':operation:'||md5((n+2)::text)||':revision:1'
            ELSE 'rh1:case:'||md5(n::text)||':operation:'||md5((n+2)::text)||':hearing:1' END,
            decode(repeat('00',32),'hex')
        FROM generate_series(0,199) n;
        ANALYZE audit_events",
        )
        .unwrap();
    let actions = "('precautionary_hearing.schedule','precautionary_hearing.replace',
        'precautionary_hearing.cancel')";
    for (sql, index) in [
        (
            format!(
                "SELECT count(*) FROM (SELECT sequence FROM audit_events
            WHERE resource='ph1:case:a:hearing:b:operation:c:revision:1'
            AND action IN {actions} LIMIT 2) matching"
            ),
            &["audit_events_action_resource", "audit_events_resource"][..],
        ),
        (
            format!(
                "SELECT EXISTS(SELECT 1 FROM audit_events a
            LEFT JOIN case_precautionary_hearing_revisions r
            ON r.audit_sequence=a.sequence AND r.hearing_id=gen_random_uuid()
            WHERE a.resource LIKE 'ph1:%:hearing:b:operation:%' AND a.action IN {actions}
            AND r.audit_sequence IS NULL)"
            ),
            &["audit_events_action_resource", "audit_events_resource"][..],
        ),
        (
            "SELECT EXISTS(SELECT 1 FROM audit_events WHERE resource LIKE 'ph1:%:operation:c:%')"
                .to_owned(),
            &["audit_events_resource"][..],
        ),
        (
            "SELECT sequence FROM audit_events WHERE action='resource_hearing.registered'
            AND resource='rh1:case:a:operation:c:hearing:1' ORDER BY sequence LIMIT 2"
                .to_owned(),
            &["audit_events_action_resource", "audit_events_resource"][..],
        ),
        (
            "SELECT EXISTS(SELECT 1 FROM audit_events WHERE action='resource_hearing.registered'
            AND resource LIKE '%:operation:c:%')"
                .to_owned(),
            &["audit_events_action_resource"][..],
        ),
    ] {
        let plan = plan(&mut db, &sql);
        assert!(!plan.contains("Seq Scan on audit_events"), "{sql}\n{plan}");
        assert!(
            index.iter().any(|name| plan.contains(name)),
            "{sql}\n{plan}"
        );
    }
}
