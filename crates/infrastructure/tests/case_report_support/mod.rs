#![allow(dead_code)]
mod atomicity;
pub use crate::case_administration_support::Fixture;
use application::{
    case_reports::*,
    cases::{case_administration_digest, CaseAdministrationValues, CaseStatusFilter},
    identity::Principal,
    ApplicationError,
};
pub use atomicity::*;
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::Clock,
    crypto::DocumentHasher,
    identity::{Role, UserId},
};
use infrastructure::{
    CaseReportEnvelopeProtector, EnvelopeKeyManager, PostgresCaseReportStore, RingAesGcmCipher,
    RingSha256Hasher,
};
use std::sync::{Arc, Mutex};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

pub struct MutableClock(Mutex<OffsetDateTime>);
impl MutableClock {
    pub fn set(&self, at: OffsetDateTime) {
        *self.0.lock().unwrap() = at;
    }
}
impl Clock for MutableClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap()
    }
}
pub fn clock(at: OffsetDateTime) -> Arc<MutableClock> {
    Arc::new(MutableClock(Mutex::new(at)))
}
pub fn fixture() -> Option<Fixture> {
    let mut db = Fixture::new()?;
    // PostgreSQL creation timestamps have microsecond precision.
    db.at = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap()
        + Duration::microseconds(123_456);
    Some(db)
}
pub fn store(db: &Fixture, clock: Arc<MutableClock>) -> PostgresCaseReportStore {
    let protector = CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        vec![0x52; 32],
    )
    .unwrap();
    PostgresCaseReportStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        clock,
        Arc::new(protector),
    )
    .unwrap()
}
pub fn principal(db: &mut Fixture, id: UserId) -> Principal {
    let row = db
        .admin
        .query_one("SELECT email,role FROM users WHERE id=$1", &[&id.as_uuid()])
        .unwrap();
    Principal {
        id,
        email: row.get("email"),
        role: row.get::<_, &str>("role").parse().unwrap(),
    }
}
pub fn owner(db: &mut Fixture) -> Principal {
    let id = db.owner;
    principal(db, id)
}
pub fn seed_case(db: &mut Fixture, title: &str, at: OffsetDateTime) -> CaseId {
    let id = CaseId::new();
    let reference = format!("REF-{id}");
    let values = CaseAdministrationValues::basic(CaseMetadata::new(title, &reference).unwrap());
    let digest = case_administration_digest(&RingSha256Hasher, &values);
    let owner_email: String = db
        .admin
        .query_one(
            "SELECT email FROM users WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap()
        .get(0);
    let mut tx = db.admin.transaction().unwrap();
    tx.query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
        .unwrap();
    tx.execute(
        "INSERT INTO cases(id,title,reference,created_by,created_at) VALUES($1,$2,$3,$4,$5::text::timestamptz)",
        &[&id.as_uuid(), &title, &reference, &db.owner.as_uuid(), &at.format(&Rfc3339).unwrap()],
    ).unwrap();
    tx.execute("INSERT INTO case_administration_revisions(case_id,revision,title,reference,administrative_status,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,1,$2,$3,'active',$4,$5,$6,$7)", &[&id.as_uuid(), &title, &reference,
        &&digest.as_bytes()[..], &at.format(&Rfc3339).unwrap(), &db.owner.as_uuid(), &owner_email]).unwrap();
    tx.commit().unwrap();
    id
}

pub fn sorted<const N: usize>(values: [CaseId; N]) -> Vec<CaseId> {
    let mut values = Vec::from(values);
    values.sort_by_key(|id| id.as_uuid());
    values
}
pub fn command(at: OffsetDateTime) -> CaseReportCommand {
    CaseReportCommand {
        operation_id: CaseReportOperationId::new(),
        filters: CaseReportFilters {
            created_from: at,
            created_before: at + Duration::days(2),
            status: CaseStatusFilter::All,
            assigned_litigator: None,
        },
    }
}
pub fn query() -> CaseReportQuery {
    CaseReportQuery {
        limit: 20,
        after_id: None,
        unread_only: false,
    }
}
pub fn request(
    store: &PostgresCaseReportStore,
    actor: &Principal,
    command: CaseReportCommand,
    at: OffsetDateTime,
) -> Result<CaseReportDetail, ApplicationError> {
    let scope = match actor.role {
        Role::Owner => CaseReportScope::Office,
        _ => CaseReportScope::AssignedCases,
    };
    let digest = case_report_request_digest(&RingSha256Hasher, actor, scope, &command)?;
    store.request(actor, scope, command, digest, at)
}
pub fn capture_request(
    store: &PostgresCaseReportStore,
    actor: &Principal,
    command: CaseReportCommand,
    at: OffsetDateTime,
) -> CaseReportSnapshot {
    let report = request(store, actor, command, at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    assert_eq!(claim.lease.report_id, report.id);
    let snapshot = store.capture(&claim.lease, at).unwrap();
    assert_eq!(
        snapshot.digest,
        case_report_snapshot_digest(&RingSha256Hasher, &snapshot).unwrap()
    );
    snapshot
}
pub fn artifacts(snapshot: &CaseReportSnapshot) -> Vec<CaseReportArtifact> {
    [CaseReportFormat::Pdf, CaseReportFormat::Csv]
        .into_iter()
        .map(|format| {
            let content = match format {
                CaseReportFormat::Pdf => b"%PDF-1.7\nreport fixture payload\n%%EOF\n".to_vec(),
                CaseReportFormat::Csv => {
                    b"kind,content\r\nreport,report fixture payload\r\n".to_vec()
                }
            };
            CaseReportArtifact {
                report_id: snapshot.report_id,
                requester: snapshot.requester.clone(),
                scope: snapshot.scope,
                format,
                snapshot_digest: snapshot.digest,
                digest: RingSha256Hasher.hash_bytes(&content),
                content,
            }
        })
        .collect()
}
pub fn finish(
    store: &PostgresCaseReportStore,
    actor: &Principal,
    command: CaseReportCommand,
    at: OffsetDateTime,
) -> CaseReportDetail {
    let report = request(store, actor, command, at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    assert_eq!(claim.lease.report_id, report.id);
    let snapshot = store.capture(&claim.lease, at).unwrap();
    store
        .complete(&claim.lease, &snapshot, artifacts(&snapshot), at)
        .unwrap()
}
pub fn count(db: &mut Fixture, table: &str) -> i64 {
    db.admin
        .query_one(&format!("SELECT count(*) FROM {table}"), &[])
        .unwrap()
        .get(0)
}
pub fn audits(db: &mut Fixture, action: &str) -> i64 {
    db.admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action=$1",
            &[&action],
        )
        .unwrap()
        .get(0)
}
