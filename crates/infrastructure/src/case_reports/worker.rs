use super::*;
use time::Duration;

impl CaseReportWorkerStore for PostgresCaseReportStore {
    fn claim_next(&self, at: OffsetDateTime) -> Result<Option<CaseReportClaim>, ApplicationError> {
        self.tx(|tx| {
            let now = self.observed(at)?;
            let row = tx.query_opt("SELECT id FROM case_report_jobs WHERE state='queued'
                OR (state IN ('retry_capturing','retry_rendering') AND retry_at::timestamptz <= $1::text::timestamptz)
                OR (state IN ('capturing','rendering') AND lease_expires_at::timestamptz <= $1::text::timestamptz)
                ORDER BY id LIMIT 1 FOR UPDATE", &[&timestamp(now)?]).map_err(port)?;
            let Some(row) = row else { return Ok(None); };
            let id = CaseReportId::from_uuid(row.get(0));
            let job = storage::get(tx, id, self.hasher.as_ref())?;
            if job.lease.as_ref().is_some_and(|lease| lease.expires_at > now)
                || matches!(job.detail.state, CaseReportState::RetryWaiting { retry_at, .. } if retry_at > now) { return Ok(None); }
            if let Err(error) = access::original(tx, &job) {
                if matches!(error, ApplicationError::CaseReport(CaseReportError::AccessRevoked)) {
                    transition_failure(tx, &job, CaseReportFailure::AccessRevoked, now)?;
                    return Ok(None);
                }
                return Err(error);
            }
            if job.attempts >= 5 || job.generation >= i64::MAX as u64 {
                transition_failure(tx, &job, CaseReportFailure::RenderFailed, now)?;
                return Ok(None);
            }
            let snapshot = match self.load_snapshot(tx, &job) {
                Ok(snapshot) => snapshot,
                Err(ApplicationError::Domain(domain::DomainError::AuthenticationFailed))
                | Err(ApplicationError::CaseReport(CaseReportError::StoredInconsistent(_)))
                | Err(ApplicationError::CaseReport(CaseReportError::CapacityExceeded)) => {
                    let failed_at = self.observed(now)?;
                    transition_failure(tx, &job, CaseReportFailure::InvalidStoredCapture, failed_at)?;
                    return Ok(None);
                }
                Err(error) => return Err(error),
            };
            let lease = CaseReportLease { report_id: id, attempt_id: CaseReportAttemptId::new(), token: CaseReportLeaseToken::new(),
                generation: job.generation + 1, expires_at: now.checked_add(Duration::minutes(2)).ok_or_else(|| inconsistent("report lease clock exhausted"))? };
            tx.execute("UPDATE case_report_jobs SET state=$2,failure=NULL,retry_at=NULL,lease_attempt=$3,lease_token=$4,
                lease_generation=$5,lease_expires_at=$6,attempts=attempts+1,updated_at=$7 WHERE id=$1",
                &[&id.as_uuid(), &if snapshot.is_some() { "rendering" } else { "capturing" }, &lease.attempt_id.as_uuid(),
                &lease.token.as_uuid(), &(lease.generation as i64), &timestamp(lease.expires_at)?, &timestamp(now)?]).map_err(port)?;
            let report = storage::get(tx, id, self.hasher.as_ref())?.detail;
            audit(tx, &report, "case_report.claim", now)?;
            Ok(Some(CaseReportClaim { lease, report, snapshot }))
        })
    }
    fn capture(
        &self,
        lease: &CaseReportLease,
        at: OffsetDateTime,
    ) -> Result<CaseReportSnapshot, ApplicationError> {
        self.capture_report(lease, at)
    }
    fn renew(
        &self,
        lease: &CaseReportLease,
        at: OffsetDateTime,
    ) -> Result<CaseReportLease, ApplicationError> {
        self.tx(|tx| {
            let job = storage::get(tx, lease.report_id, self.hasher.as_ref())?;
            let now = self.observed(at)?;
            storage::fence(&job, lease, now)?;
            access::original(tx, &job)?;
            let mut current = lease.clone();
            current.expires_at = lease
                .expires_at
                .checked_add(Duration::minutes(2))
                .ok_or_else(|| inconsistent("report lease clock exhausted"))?;
            tx.execute(
                "UPDATE case_report_jobs SET lease_expires_at=$2,updated_at=$3 WHERE id=$1",
                &[
                    &lease.report_id.as_uuid(),
                    &timestamp(current.expires_at)?,
                    &timestamp(now)?,
                ],
            )
            .map_err(port)?;
            audit(tx, &job.detail, "case_report.renew", now)?;
            Ok(current)
        })
    }
    fn complete(
        &self,
        lease: &CaseReportLease,
        snapshot: &CaseReportSnapshot,
        artifacts: Vec<CaseReportArtifact>,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError> {
        self.complete_report(lease, snapshot, artifacts, at)
    }
    fn fail(
        &self,
        lease: &CaseReportLease,
        failure: CaseReportFailure,
        at: OffsetDateTime,
    ) -> Result<CaseReportWorkerRun, ApplicationError> {
        self.tx(|tx| {
            let job = storage::get(tx, lease.report_id, self.hasher.as_ref())?;
            let now = self.observed(at)?;
            storage::fence(&job, lease, now)?;
            let actual = match access::original(tx, &job) {
                Ok(()) => failure,
                Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked)) => {
                    CaseReportFailure::AccessRevoked
                }
                Err(error) => return Err(error),
            };
            transition_failure(tx, &job, actual, now)
        })
    }
}
fn transition_failure(
    tx: &mut Transaction<'_>,
    job: &storage::Job,
    failure: CaseReportFailure,
    now: OffsetDateTime,
) -> Result<CaseReportWorkerRun, ApplicationError> {
    let id = job.detail.id;
    let has_snapshot = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_report_snapshots WHERE report_id=$1)",
            &[&id.as_uuid()],
        )
        .map_err(port)?
        .get::<_, bool>(0);
    let retry = job.attempts < 5
        && matches!(
            failure,
            CaseReportFailure::TemporaryUnavailable | CaseReportFailure::RenderUnavailable
        );
    let state = if failure == CaseReportFailure::AccessRevoked {
        "revoked"
    } else if retry {
        if has_snapshot {
            "retry_rendering"
        } else {
            "retry_capturing"
        }
    } else {
        "failed"
    };
    let retry_at = if retry {
        Some(timestamp(
            now.checked_add(Duration::seconds(30 * i64::from(job.attempts.max(1))))
                .ok_or_else(|| inconsistent("report retry clock exhausted"))?,
        )?)
    } else {
        None
    };
    tx.execute("UPDATE case_report_jobs SET state=$2,failure=$3,retry_at=$4,lease_attempt=NULL,lease_token=NULL,
        lease_expires_at=NULL,updated_at=$5 WHERE id=$1", &[&id.as_uuid(), &state, &codec::failure_name(failure), &retry_at, &timestamp(now)?]).map_err(port)?;
    if state == "failed" {
        tx.execute(
            "INSERT INTO case_report_notices(report_id,kind,created_at) VALUES($1,'failed',$2)",
            &[&id.as_uuid(), &timestamp(now)?],
        )
        .map_err(port)?;
    }
    audit(tx, &job.detail, "case_report.fail", now)?;
    Ok(if state == "revoked" {
        CaseReportWorkerRun::AccessRevoked(id)
    } else if retry {
        CaseReportWorkerRun::Deferred(id)
    } else {
        CaseReportWorkerRun::Failed(id)
    })
}
