use super::{checks::*, *};
use crate::{ApplicationError, PortFailureKind};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
};
use std::sync::Arc;

pub struct CaseReportWorker {
    store: Arc<dyn CaseReportWorkerStore>,
    renderer: Arc<dyn CaseReportRenderer>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl CaseReportWorker {
    pub fn new(
        store: Arc<dyn CaseReportWorkerStore>,
        renderer: Arc<dyn CaseReportRenderer>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            renderer,
            hasher,
            clock,
        }
    }
    pub fn run_next(&self) -> Result<CaseReportWorkerRun, ApplicationError> {
        let started = self.clock.now();
        time(started)?;
        let Some(claim) = self.store.claim_next(started)? else {
            return Ok(CaseReportWorkerRun::Idle);
        };
        let mut at = self.observed(started)?;
        lease(&claim.lease, at)?;
        super::receipt::detail(
            self.hasher.as_ref(),
            &claim.report,
            &claim.report.requester.principal,
            claim.lease.report_id,
            at,
        )?;
        if !matches!(
            claim.report.state,
            CaseReportState::Processing(CaseReportPhase::Capturing | CaseReportPhase::Rendering)
        ) {
            return Err(inconsistent("claimed report is not processing"));
        }
        if claim.snapshot.is_some()
            != matches!(
                claim.report.state,
                CaseReportState::Processing(CaseReportPhase::Rendering)
            )
        {
            return Err(inconsistent(
                "claimed report phase differs from capture presence",
            ));
        }
        let mut lease = claim.lease;
        let snapshot = match claim.snapshot {
            Some(snapshot) => snapshot,
            None => match self.store.capture(&lease, at) {
                Ok(snapshot) => snapshot,
                Err(error) => return self.capture_failure(&lease, error, at),
            },
        };
        at = self.observed(at)?;
        checks::lease(&lease, at)?;
        if let Err(error) =
            super::receipt::capture(self.hasher.as_ref(), &snapshot, &claim.report, at)
        {
            let failure = if matches!(
                error,
                ApplicationError::CaseReport(CaseReportError::CapacityExceeded)
            ) {
                CaseReportFailure::CapacityExceeded
            } else {
                CaseReportFailure::InvalidStoredCapture
            };
            return self.fail(&lease, failure, at);
        }
        let mut artifacts = Vec::with_capacity(2);
        for format in [CaseReportFormat::Pdf, CaseReportFormat::Csv] {
            at = self.observed(at)?;
            checks::lease(&lease, at)?;
            let renewed = match self.store.renew(&lease, at) {
                Ok(value) => value,
                Err(error) => return self.capture_failure(&lease, error, at),
            };
            at = self.observed(at)?;
            renewal(&lease, &renewed, at)?;
            lease = renewed;
            let rendered = self.renderer.render(&snapshot, format);
            at = self.observed(at)?;
            checks::lease(&lease, at)?;
            let content = match rendered {
                Ok(content) => content,
                Err(error) => {
                    let reason = match error {
                        ApplicationError::CaseReport(CaseReportError::RenderUnavailable)
                        | ApplicationError::ClassifiedPort {
                            kind: PortFailureKind::Unavailable,
                            ..
                        } => CaseReportFailure::RenderUnavailable,
                        ApplicationError::CaseReport(CaseReportError::CapacityExceeded) => {
                            CaseReportFailure::CapacityExceeded
                        }
                        _ => CaseReportFailure::RenderFailed,
                    };
                    return self.fail(&lease, reason, at);
                }
            };
            if content.len() > MAX_REPORT_ARTIFACT_BYTES {
                return self.fail(&lease, CaseReportFailure::CapacityExceeded, at);
            }
            if content.is_empty() {
                return self.fail(&lease, CaseReportFailure::RenderFailed, at);
            }
            artifacts.push(CaseReportArtifact {
                report_id: claim.report.id,
                requester: snapshot.requester.clone(),
                scope: snapshot.scope,
                format,
                snapshot_digest: snapshot.digest,
                digest: self.hasher.hash_bytes(&content),
                content,
            });
        }
        let metadata: Vec<_> = artifacts
            .iter()
            .map(|v| CaseReportArtifactMetadata {
                format: v.format,
                bytes: v.content.len() as u64,
                digest: v.digest,
            })
            .collect();
        at = self.observed(at)?;
        checks::lease(&lease, at)?;
        let result = self.store.complete(&lease, &snapshot, artifacts, at)?;
        let returned = self.observed(at)?;
        super::receipt::completion(
            self.hasher.as_ref(),
            &result,
            &claim.report,
            &snapshot,
            &metadata,
            &lease,
            returned,
        )?;
        Ok(CaseReportWorkerRun::Ready(claim.report.id))
    }
    fn observed(&self, previous: OffsetDateTime) -> Result<OffsetDateTime, ApplicationError> {
        let observed = self.clock.now();
        window(previous, observed)?;
        Ok(observed)
    }
    fn capture_failure(
        &self,
        lease: &CaseReportLease,
        error: ApplicationError,
        previous: OffsetDateTime,
    ) -> Result<CaseReportWorkerRun, ApplicationError> {
        let failure = match error {
            ApplicationError::CaseReport(CaseReportError::LeaseLost) => {
                return Err(CaseReportError::LeaseLost.into())
            }
            ApplicationError::CaseReport(CaseReportError::AccessRevoked) => {
                CaseReportFailure::AccessRevoked
            }
            ApplicationError::CaseReport(CaseReportError::CapacityExceeded) => {
                CaseReportFailure::CapacityExceeded
            }
            ApplicationError::Port(_) | ApplicationError::ClassifiedPort { .. } => {
                CaseReportFailure::TemporaryUnavailable
            }
            _ => CaseReportFailure::InvalidStoredCapture,
        };
        self.fail(lease, failure, self.observed(previous)?)
    }
    fn fail(
        &self,
        lease: &CaseReportLease,
        failure: CaseReportFailure,
        at: OffsetDateTime,
    ) -> Result<CaseReportWorkerRun, ApplicationError> {
        checks::lease(lease, at)?;
        let result = self.store.fail(lease, failure, at)?;
        self.observed(at)?;
        let valid = match result {
            CaseReportWorkerRun::Failed(id) => {
                id == lease.report_id && failure != CaseReportFailure::AccessRevoked
            }
            CaseReportWorkerRun::AccessRevoked(id) => {
                id == lease.report_id && failure == CaseReportFailure::AccessRevoked
            }
            CaseReportWorkerRun::Deferred(id) => {
                id == lease.report_id
                    && matches!(
                        failure,
                        CaseReportFailure::TemporaryUnavailable
                            | CaseReportFailure::RenderUnavailable
                    )
            }
            _ => false,
        };
        if !valid {
            return Err(inconsistent(
                "report failure result differs from fenced transition",
            ));
        }
        Ok(result)
    }
}
fn renewal(
    previous: &CaseReportLease,
    current: &CaseReportLease,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if current.report_id != previous.report_id
        || current.attempt_id != previous.attempt_id
        || current.token != previous.token
        || current.generation != previous.generation
        || current.expires_at <= previous.expires_at
    {
        return Err(inconsistent(
            "report renewal changed fencing identity or did not extend expiry",
        ));
    }
    checks::lease(current, at)
}
