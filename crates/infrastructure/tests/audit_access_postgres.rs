use crate::audit_query_support::*;
use application::access_audit::{AccessAuditRecord, AccessAuditService, AccessAuditWriter};
use domain::{
    audit::{AuditContext, AuditSource, ChainedEvent},
    clock::Clock,
};
use infrastructure::SystemClock;
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Write, path::PathBuf, sync::Arc};
use uuid::Uuid;

#[path = "audit_access_measurement_support.rs"]
mod measurement;
use measurement::{Sample, HISTORY, LIMIT_NS, OUTCOMES, REPETITIONS};

fn service(db: &Fixture) -> Arc<AccessAuditService> {
    Arc::new(AccessAuditService::new(
        Box::new(PostgresAuditLog::open(&db.runtime_url).unwrap()),
        Arc::new(SystemClock),
    ))
}

fn context(user_id: Option<domain::identity::UserId>) -> AuditContext {
    AuditContext {
        user_id,
        ip_address: Some("192.0.2.80".parse().unwrap()),
        source: AuditSource::Http,
        request_id: Some(Uuid::new_v4()),
    }
}

#[test]
fn native_access_service_commits_all_outcomes_and_keeps_unavailable_identity_explicit() {
    let Some(db) = Fixture::new() else { return };
    let writer = service(&db);
    let mut expected = Vec::new();
    for action in OUTCOMES {
        let context = context(if action == "access.denied" {
            None
        } else {
            Some(db.owner)
        });
        writer
            .record(AccessAuditRecord {
                context: context.clone(),
                action: action.into(),
                resource: "GET /api/v1/cases/:id".into(),
            })
            .unwrap();
        expected.push((action, context));
    }
    drop(writer);
    let restored = trail(&db);
    assert_eq!(restored.len(), expected.len());
    for (entry, (action, context)) in restored.iter().zip(expected) {
        assert_eq!(entry.event.action, action);
        assert_eq!(
            entry.event.actor,
            context
                .user_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unavailable".into())
        );
        assert_eq!(entry.event.context.as_ref(), Some(&context));
    }
    assert_chain(&db);
}

#[test]
fn native_access_history_denies_other_roles_and_a_revoked_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let writer = service(&db);
    writer
        .record(AccessAuditRecord {
            context: context(Some(db.owner)),
            action: "access.granted".into(),
            resource: "GET /api/v1/audit/events".into(),
        })
        .unwrap();
    let at = SystemClock.now();
    let query = AuditEventQuery::new(
        at - Duration::days(1),
        at + Duration::days(1),
        None,
        Some("access.granted"),
        None,
        20,
        None,
    )
    .unwrap();
    let reader = store(&db);
    let page = reader.read(&owner(&db), &query, at).unwrap();
    assert_eq!(page.events.len(), 1);
    let staff_id = db.user("litigator", false);
    let staff = Principal {
        id: staff_id,
        email: format!("{staff_id}@example.test"),
        role: Role::Litigator,
    };
    db.user("owner", false);
    let before = trail(&db);
    assert!(matches!(
        reader.read(&staff, &query, at),
        Err(ApplicationError::PermissionDenied)
    ));
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&db.owner.as_uuid()]).unwrap();
    assert!(matches!(
        reader.read(&owner(&db), &query, at),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(trail(&db), before);
}

fn seed(db: &Fixture) -> Vec<ChainedEvent> {
    let mut log = PostgresAuditLog::open(&db.runtime_url).unwrap();
    for index in 0..HISTORY {
        log.append(
            "history worker",
            "internal.fixture",
            &format!("history-{index:04}"),
            db.at + Duration::nanoseconds(index as i64),
        )
        .unwrap();
    }
    let events = log.load_all().unwrap();
    assert_eq!(events.len(), HISTORY);
    assert_eq!(
        domain::audit::verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid { entries: HISTORY }
    );
    events
}

fn postgres_environment(db: &mut Fixture) -> Value {
    let row = db
        .runtime()
        .query_one(
            "SELECT current_setting('server_version'),
        current_setting('fsync'),current_setting('full_page_writes'),
        current_setting('synchronous_commit'),current_setting('wal_sync_method')",
            &[],
        )
        .unwrap();
    for index in [1, 2, 3] {
        assert_eq!(
            row.get::<_, String>(index),
            "on",
            "durable measurement requires enabled PostgreSQL durability"
        );
    }
    let size: i64 = db
        .admin
        .query_one("SELECT pg_total_relation_size('audit_events')", &[])
        .unwrap()
        .get(0);
    json!({"version": row.get::<_, String>(0), "fsync": row.get::<_, String>(1),
        "full_page_writes": row.get::<_, String>(2), "synchronous_commit": row.get::<_, String>(3),
        "wal_sync_method": row.get::<_, String>(4), "initial_history_rows": HISTORY,
        "initial_history_relation_bytes": size, "writer_connections": 1})
}

fn verify_receipts(db: &Fixture, original: &[ChainedEvent], samples: &mut [Sample]) -> Value {
    let restored = trail(db);
    let mut mapping: BTreeMap<_, _> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| (sample.context.request_id.unwrap(), index))
        .collect();
    let mut matches = true;
    for entry in restored.iter().skip(HISTORY) {
        let Some(context) = &entry.event.context else {
            matches = false;
            continue;
        };
        let Some(index) = context.request_id.and_then(|id| mapping.remove(&id)) else {
            matches = false;
            continue;
        };
        let sample = &mut samples[index];
        matches &= sample.durable
            && sample.context == *context
            && sample.action == entry.event.action
            && entry.event.resource == format!("{} {}", sample.method, sample.path)
            && entry.event.actor
                == context
                    .user_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unavailable".into());
        sample.audit_sequence = Some(entry.event.sequence);
        sample.audit_chain = Some(entry.chain.to_hex());
    }
    matches &= mapping.values().all(|index| !samples[*index].durable);
    let chain_valid = domain::audit::verify_chain(&RingSha256Hasher, &restored).unwrap()
        == (ChainVerification::Valid {
            entries: restored.len(),
        });
    let prefix_unchanged = restored.starts_with(original);
    let exact_count = restored.len() == HISTORY + samples.iter().filter(|s| s.durable).count();
    json!({"reopened_connection": true, "chain_valid": chain_valid,
        "prefix_unchanged": prefix_unchanged, "request_mapping_exact": matches,
        "event_count_exact": exact_count, "final_history_rows": restored.len(),
        "passed": chain_valid && prefix_unchanged && matches && exact_count})
}

#[test]
#[ignore = "manual durable access measurement requires isolated PostgreSQL and declared environment"]
fn durable_access_catalog_latency_with_bounded_workers() {
    let revision =
        std::env::var("AUDIT_ACCESS_REVISION").expect("declare measured source revision");
    assert!(!revision.trim().is_empty());
    let environment: Value = serde_json::from_str(
        &std::env::var("AUDIT_ACCESS_ENVIRONMENT")
            .expect("declare measured host and storage environment as JSON"),
    )
    .unwrap();
    for field in [
        "os",
        "cpu",
        "memory_bytes",
        "postgres_disk",
        "build_profile",
        "temporary_filesystem",
    ] {
        assert!(
            environment.get(field).is_some(),
            "missing environment field: {field}"
        );
    }
    let destination =
        PathBuf::from(std::env::var("AUDIT_ACCESS_RECEIPT").expect("declare a new receipt path"));
    assert!(
        destination.is_absolute() && !destination.exists(),
        "receipt must be a new absolute path"
    );
    let routes = measurement::catalog();
    let mut populations = Vec::new();
    let mut passed = true;
    for concurrency in [1, 8] {
        let mut db =
            Fixture::new().expect("native access measurement requires CASE_TEST_DATABASE_URL");
        let original = seed(&db);
        let postgres = postgres_environment(&mut db);
        let writer = service(&db);
        let mut samples = measurement::measure(writer.clone(), db.owner, concurrency, &routes);
        drop(writer);
        let verification = verify_receipts(&db, &original, &mut samples);
        passed &= verification["passed"] == true
            && samples
                .iter()
                .all(|sample| sample.durable && sample.elapsed_ns < LIMIT_NS);
        let summary = measurement::distribution(&samples);
        populations.push(json!({"concurrency": concurrency, "postgres": postgres,
            "verification": verification, "summary": summary, "samples": samples}));
    }
    let receipt = json!({"format": "qadra-native-access-measurement", "version": 1,
        "status": if passed { "passed" } else { "failed" }, "revision": revision,
        "environment": environment, "functional_routes": routes.len(),
        "outcomes": OUTCOMES, "repetitions": REPETITIONS, "planned_samples": 5328,
        "population": "direct AccessAuditService; finite route/outcome payloads; no HTTP permission exercise",
        "timer": "before bounded worker enqueue through durable record return; includes queue, writer mutex, SQL lock and commit",
        "excluded_timing": "fixture seeding, connection validation, HTTP and Tokio blocking pool",
        "cold_definition": "first batch on a fresh writer; PostgreSQL and OS caches are not claimed cold",
        "populations": populations});
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options.open(destination).unwrap();
    serde_json::to_writer_pretty(&mut output, &receipt).unwrap();
    output.write_all(b"\n").unwrap();
    output.sync_all().unwrap();
    assert!(
        passed,
        "durable access acceptance failed; individual samples were preserved"
    );
}
