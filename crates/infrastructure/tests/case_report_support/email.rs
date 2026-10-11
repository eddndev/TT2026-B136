use crate::case_report_support::*;
use application::{case_reports::*, identity::Principal};
use domain::clock::OffsetDateTime;
use infrastructure::{
    CaseReportEnvelopeProtector, EnvelopeKeyManager, PostgresCaseReportStore, RingAesGcmCipher,
    RingSha256Hasher,
};
use serde_json::Value;
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, Duration};
use uuid::Uuid;

#[path = "../member_invitation_restore/backup.rs"]
pub(crate) mod backup;
#[path = "email_inventory.rs"]
mod inventory;
#[path = "email_leases.rs"]
mod leases;
#[path = "email_publication.rs"]
mod publication;
#[path = "email_restore.rs"]
mod restore;
#[path = "email_worker.rs"]
mod worker;

fn template() -> CaseReportEmailTemplate {
    CaseReportEmailTemplate {
        from: "reports@example.test".into(),
        subject: "Qadra: informe disponible".into(),
        text: "Tu informe esta disponible. Inicia sesion en https://qadra.example.test/#reports"
            .into(),
    }
}

fn claim(at: OffsetDateTime) -> CaseReportEmailClaim {
    CaseReportEmailClaim {
        lease_id: Uuid::new_v4(),
        at,
        lease_seconds: 30,
        provider_window_seconds: 86400,
    }
}

fn outcome(
    lease: &CaseReportEmailLease,
    state: CaseReportEmailState,
    retry_at: Option<OffsetDateTime>,
    at: OffsetDateTime,
) -> CaseReportEmailFinish {
    CaseReportEmailFinish {
        report_id: lease.report_id,
        delivery_id: lease.delivery_id,
        generation: lease.generation,
        lease_id: lease.lease_id,
        state,
        retry_at,
        at,
    }
}

fn estimated_request(
    db: &mut Fixture,
    store: &PostgresCaseReportStore,
    actor: &Principal,
    milliseconds: Option<i64>,
) -> CaseReportDetail {
    let source = finish(store, actor, command(db.at), db.at);
    let id = CaseReportId::new();
    let mut command = source.command.clone();
    command.operation_id = CaseReportOperationId::new();
    let digest =
        case_report_request_digest(&RingSha256Hasher, actor, source.scope, &command).unwrap();
    // Insert a new valid request; the original request and its estimate stay immutable.
    db.admin.execute(
        "INSERT INTO case_report_jobs(id,requester_id,principal,account_revision,auth_generation,
         scope,operation_id,command,request_digest,requested_at,updated_at,state,estimated_total_ms)
         SELECT $1,requester_id,principal,account_revision,auth_generation,scope,$2,
         jsonb_set(command,'{operation_id}',to_jsonb($2::uuid::text)),$3,$4,$4,'queued',$5
         FROM case_report_jobs WHERE id=$6",
        &[&id.as_uuid(), &command.operation_id.as_uuid(), &&digest.as_bytes()[..],
          &db.at.format(&Rfc3339).unwrap(), &milliseconds, &source.id.as_uuid()],
    ).unwrap();
    let saved = store.get(actor, id, db.at).unwrap();
    assert_eq!(saved.command, command);
    saved
}

fn publish(
    store: &PostgresCaseReportStore,
    requested: &CaseReportDetail,
    at: OffsetDateTime,
) -> CaseReportDetail {
    let claim = store.claim_next(at).unwrap().unwrap();
    assert_eq!(claim.report.id, requested.id);
    let snapshot = store.capture(&claim.lease, at).unwrap();
    store
        .complete(&claim.lease, &snapshot, artifacts(&snapshot), at)
        .unwrap()
}

fn ready(db: &mut Fixture, store: &PostgresCaseReportStore, actor: &Principal) -> CaseReportDetail {
    let request = estimated_request(db, store, actor, Some(4000));
    publish(store, &request, db.at)
}

fn delivery(db: &mut Fixture, id: CaseReportId) -> Option<Value> {
    db.admin
        .query_opt(
            "SELECT to_jsonb(d) FROM case_report_email_deliveries d WHERE report_id=$1",
            &[&id.as_uuid()],
        )
        .unwrap()
        .map(|row| row.get(0))
}

fn open_url(url: &str, timer: Arc<MutableClock>) -> PostgresCaseReportStore {
    let protector = CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        vec![0x52; 32],
    )
    .unwrap();
    PostgresCaseReportStore::open(url, Arc::new(RingSha256Hasher), timer, Arc::new(protector))
        .unwrap()
}
