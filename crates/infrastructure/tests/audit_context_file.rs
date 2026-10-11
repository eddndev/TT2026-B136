use application::audit_context::AuditScope;
use domain::audit::{verify_chain, AuditContext, AuditLog, AuditSource, ChainVerification};
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use infrastructure::{FileAuditLog, RingSha256Hasher};
use time::OffsetDateTime;
use uuid::Uuid;

fn context() -> AuditContext {
    AuditContext {
        user_id: Some(UserId::from_uuid(
            Uuid::parse_str("10000000-0000-4000-8000-000000000051").unwrap(),
        )),
        ip_address: Some("192.0.2.51".parse().unwrap()),
        source: AuditSource::Http,
        request_id: Some(Uuid::parse_str("10000000-0000-4000-8000-000000000052").unwrap()),
    }
}

#[test]
fn file_round_trip_preserves_context_and_context_tampering_breaks_the_chain() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("audit.jsonl");
    let expected = context();
    let scope = AuditScope::new(expected.clone());
    let at = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
    let mut writer = FileAuditLog::new(&path, RingSha256Hasher);
    let recorded = scope
        .run(|| writer.append("owner@example.test", "document.read", "case", at))
        .unwrap();
    assert_eq!(recorded.event.context, Some(expected.clone()));
    drop(writer);
    let reopened = FileAuditLog::new(&path, RingSha256Hasher);
    assert_eq!(reopened.load_all().unwrap(), vec![recorded.clone()]);
    assert_eq!(
        verify_chain(&RingSha256Hasher, &reopened.load_all().unwrap()).unwrap(),
        ChainVerification::Valid { entries: 1 }
    );
    let mut stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(stored["context"], serde_json::to_value(&expected).unwrap());
    let changed = AuditContext {
        ip_address: Some("192.0.2.52".parse().unwrap()),
        ..expected
    };
    stored["context"] = serde_json::to_value(&changed).unwrap();
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string(&stored).unwrap()),
    )
    .unwrap();
    let altered = reopened.load_all().unwrap();
    assert_eq!(altered[0].chain, recorded.chain);
    assert_eq!(altered[0].event.context, Some(changed));
    assert_eq!(
        verify_chain(&RingSha256Hasher, &altered).unwrap(),
        ChainVerification::Broken {
            first_broken_index: 0
        }
    );
}

#[test]
fn appending_context_preserves_legacy_file_bytes_and_absent_historical_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("audit.jsonl");
    let timestamp = "2025-01-01T00:00:00Z";
    let mut canonical = 0u64.to_be_bytes().to_vec();
    for field in [timestamp, "old actor", "old.read", "case-old"] {
        canonical.extend_from_slice(&(field.len() as u32).to_be_bytes());
        canonical.extend_from_slice(field.as_bytes());
    }
    let mut linked = vec![0u8; 32];
    linked.extend_from_slice(&canonical);
    let chain = RingSha256Hasher.hash_bytes(&linked);
    let legacy = serde_json::json!({
        "sequence": 0, "timestamp": timestamp, "actor": "old actor",
        "action": "old.read", "resource": "case-old", "chain": chain.to_hex()
    });
    let original = format!("{}\n", serde_json::to_string(&legacy).unwrap()).into_bytes();
    std::fs::write(&path, &original).unwrap();
    let mut log = FileAuditLog::new(&path, RingSha256Hasher);
    let historical = log.load_all().unwrap();
    assert_eq!(historical[0].event.context, None);
    assert_eq!(historical[0].event.canonical_bytes().unwrap(), canonical);
    let expected = context();
    AuditScope::new(expected.clone())
        .run(|| {
            log.append(
                "owner@example.test",
                "document.read",
                "case",
                OffsetDateTime::from_unix_timestamp(1_735_689_601).unwrap(),
            )
        })
        .unwrap();
    assert!(std::fs::read(&path).unwrap().starts_with(&original));
    let restored = FileAuditLog::new(&path, RingSha256Hasher)
        .load_all()
        .unwrap();
    assert_eq!(restored[0], historical[0]);
    assert_eq!(restored[1].event.context, Some(expected));
    assert_eq!(
        verify_chain(&RingSha256Hasher, &restored).unwrap(),
        ChainVerification::Valid { entries: 2 }
    );
}
