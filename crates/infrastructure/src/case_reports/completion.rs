use super::*;

impl PostgresCaseReportStore {
    pub(super) fn complete_report(
        &self,
        lease: &CaseReportLease,
        snapshot: &CaseReportSnapshot,
        artifacts: Vec<CaseReportArtifact>,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError> {
        validate_case_report_snapshot(self.hasher.as_ref(), snapshot)?;
        if artifacts.len() != 2 || artifacts[0].format == artifacts[1].format {
            return Err(inconsistent(
                "report completion requires exactly PDF and CSV",
            ));
        }
        for artifact in &artifacts {
            if artifact.report_id != snapshot.report_id
                || artifact.requester != snapshot.requester
                || artifact.scope != snapshot.scope
                || artifact.snapshot_digest != snapshot.digest
                || artifact.content.is_empty()
                || artifact.content.len() > MAX_REPORT_ARTIFACT_BYTES
                || self.hasher.hash_bytes(&artifact.content) != artifact.digest
            {
                return Err(inconsistent(
                    "report artifact does not match its capture or content digest",
                ));
            }
        }
        self.tx(|tx| {
            let job = storage::get(tx, lease.report_id, self.hasher.as_ref())?;
            let started = self.observed(at)?;
            storage::fence(&job, lease, started)?;
            access::original(tx, &job)?;
            let saved = self.load_snapshot(tx, &job)?.ok_or(CaseReportError::NotReady)?;
            if saved != *snapshot || job.detail.state != CaseReportState::Processing(CaseReportPhase::Rendering) {
                return Err(inconsistent("completed capture differs from immutable saved snapshot"));
            }
            for artifact in &artifacts { self.save_artifact(tx, artifact)?; }
            let now = self.observed(started)?;
            storage::fence(&job, lease, now)?;
            tx.execute("UPDATE case_report_jobs SET state='ready',failure=NULL,retry_at=NULL,lease_attempt=NULL,
                lease_token=NULL,lease_expires_at=NULL,updated_at=$2 WHERE id=$1", &[&job.detail.id.as_uuid(), &timestamp(now)?]).map_err(port)?;
            tx.execute("INSERT INTO case_report_notices(report_id,kind,created_at) VALUES($1,'ready',$2)", &[&job.detail.id.as_uuid(), &timestamp(now)?]).map_err(port)?;
            audit(tx, &job.detail, "case_report.complete", now)?;
            Ok(storage::get(tx, job.detail.id, self.hasher.as_ref())?.detail)
        })
    }
}
