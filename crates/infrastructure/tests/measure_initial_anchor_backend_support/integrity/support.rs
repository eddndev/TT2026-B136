use super::*;
use application::ApplicationError;
use domain::{
    audit::{chain_digest, AuditEvent},
    crypto::Sha256Digest,
};

pub(super) fn open(db: &Fixture) -> Result<PostgresMeasureDecisionStore, ApplicationError> {
    PostgresMeasureDecisionStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub(super) fn damage(client: &mut Client, tables: &[&str], statement: &str) {
    let mut tx = client.transaction().unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.batch_execute(statement).unwrap();
    for table in tables {
        tx.batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
            .unwrap();
    }
    tx.commit().unwrap();
}

pub(super) fn reject_original(
    storage: &PostgresMeasureDecisionStore,
    actor: &Principal,
    case: CaseId,
    command: &MeasureDecisionCommand,
) {
    assert!(storage.get(actor, case, command.decision_id).is_err());
    assert!(storage
        .get_operation(actor, case, command.operation_id)
        .is_err());
    assert!(storage
        .prepare(actor, case, command, &StageSupportReadLimits::default())
        .is_err());
}

pub(super) fn marker(hearing: &HearingDetail) -> String {
    let snapshot = &hearing.snapshot;
    format!(
        "case:{}:hearing:{}:revision:{}:operation:{}:sha256:{}",
        snapshot.case_id,
        snapshot.id,
        snapshot.revision.get(),
        snapshot.receipt.operation_id,
        snapshot.receipt.submission_digest.to_hex()
    )
}

pub(super) fn duplicate_audit(db: &Fixture, hearing: &HearingDetail) {
    let mut client = db.runtime();
    let mut tx = client.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    let original = tx
        .query_one(
            "SELECT timestamp,actor,action,resource FROM audit_events WHERE resource=$1",
            &[&marker(hearing)],
        )
        .unwrap();
    let head = tx
        .query_one(
            "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
            &[],
        )
        .unwrap();
    let sequence = head.get::<_, i64>(0) + 1;
    let previous = Sha256Digest::from_bytes(&head.get::<_, Vec<u8>>(1)).unwrap();
    let at = time::OffsetDateTime::parse(
        &original.get::<_, String>(0),
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap();
    let event = AuditEvent::new(
        sequence as u64,
        at,
        original.get::<_, String>(1),
        original.get::<_, String>(2),
        original.get::<_, String>(3),
    );
    assert_eq!(event.action, "hearing.scheduled");
    assert_eq!(event.resource, marker(hearing));
    let chain = chain_digest(&RingSha256Hasher, &previous, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action,
            &event.resource, &chain.as_bytes().as_slice()]).unwrap();
    tx.commit().unwrap();
}
