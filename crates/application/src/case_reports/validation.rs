use super::{canonical::Encoder, checks::*, *};
use crate::{identity::Principal, ApplicationError};
use domain::crypto::{DocumentHasher, Sha256Digest};

pub fn case_report_request_digest(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    scope: CaseReportScope,
    command: &CaseReportCommand,
) -> Result<Sha256Digest, ApplicationError> {
    principal(actor, scope)?;
    filters(&command.filters)?;
    if command.operation_id.as_uuid().is_nil() {
        return Err(ApplicationError::InvalidInput(
            "report operation identity is absent".into(),
        ));
    }
    let mut bytes = Encoder::new(b"tt.case-report.request")?;
    bytes.principal(actor)?;
    bytes.scope(scope)?;
    bytes.uuid(command.operation_id.as_uuid())?;
    bytes.filters(&command.filters)?;
    Ok(hasher.hash_bytes(&bytes.finish()))
}
pub fn case_report_snapshot_digest(
    hasher: &dyn DocumentHasher,
    snapshot: &CaseReportSnapshot,
) -> Result<Sha256Digest, ApplicationError> {
    super::snapshot::validate(snapshot)?;
    let mut bytes = Encoder::new(b"tt.case-report.snapshot")?;
    bytes.uuid(snapshot.report_id.as_uuid())?;
    bytes.requester(&snapshot.requester)?;
    bytes.scope(snapshot.scope)?;
    bytes.filters(&snapshot.filters)?;
    bytes.time(snapshot.checked_at)?;
    bytes.number(snapshot.cases.len() as u64)?;
    for case in &snapshot.cases {
        bytes.uuid(case.case_id.as_uuid())?;
        bytes.bytes(case.title.as_bytes())?;
        bytes.bytes(case.reference.as_bytes())?;
        bytes.time(case.created_at)?;
        bytes.bytes(case.status.as_str().as_bytes())?;
        bytes.number(u64::from(case.administration_revision.is_some()))?;
        if let (Some(revision), Some(digest)) =
            (case.administration_revision, case.administration_digest)
        {
            bytes.number(u64::from(revision.get()))?;
            bytes.digest(digest)?;
        }
        bytes.number(case.assigned_litigators.len() as u64)?;
        for who in &case.assigned_litigators {
            bytes.litigator(who)?;
        }
    }
    bytes.number(snapshot.workload.len() as u64)?;
    for row in &snapshot.workload {
        bytes.litigator(&row.litigator)?;
        bytes.number(row.active_cases)?;
        bytes.number(row.closed_cases)?;
    }
    Ok(hasher.hash_bytes(&bytes.finish()))
}
pub fn validate_case_report_snapshot(
    hasher: &dyn DocumentHasher,
    snapshot: &CaseReportSnapshot,
) -> Result<(), ApplicationError> {
    if case_report_snapshot_digest(hasher, snapshot)? != snapshot.digest {
        return Err(inconsistent(
            "snapshot digest does not match its captured contents",
        ));
    }
    Ok(())
}
