//! One leased ready-report email attempt, independent of artifact generation.

use std::sync::Arc;

use domain::{
    clock::{Clock, OffsetDateTime},
    typed_participants::Uuid,
};
use time::Duration;

use super::{
    checks, CaseReportEmailClaim, CaseReportEmailDelivery, CaseReportEmailFinish,
    CaseReportEmailLease, CaseReportEmailOutcome, CaseReportEmailRun, CaseReportEmailState,
    CaseReportEmailStore, CaseReportEmailTemplate,
};
use crate::ApplicationError;

const LEASE_SECONDS: u32 = 30;
const PROVIDER_WINDOW_SECONDS: u32 = 86_400;
const RETRY_DELAY_SECONDS: i64 = 60;

pub struct CaseReportEmailWorker {
    store: Arc<dyn CaseReportEmailStore>,
    delivery: Arc<dyn CaseReportEmailDelivery>,
    template: CaseReportEmailTemplate,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl CaseReportEmailWorker {
    pub fn new(
        store: Arc<dyn CaseReportEmailStore>,
        delivery: Arc<dyn CaseReportEmailDelivery>,
        template: CaseReportEmailTemplate,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            delivery,
            template,
            clock,
        }
    }

    /// Performs at most one provider attempt; durable state owns later retries.
    pub fn run_once(&self) -> Result<CaseReportEmailRun, ApplicationError> {
        let started = self.clock.now();
        checks::time(started)?;
        let lease_id = Uuid::new_v4();
        let claim = self
            .store
            .claim_delivery(
                CaseReportEmailClaim {
                    lease_id,
                    at: started,
                    lease_seconds: LEASE_SECONDS,
                    provider_window_seconds: PROVIDER_WINDOW_SECONDS,
                },
                &self.template,
            )
            .map_err(|_| unavailable("report email claim failed"))?;
        let at = self.observed(started)?;
        let Some(lease) = claim else {
            return Ok(CaseReportEmailRun::Idle);
        };
        validate_lease(&lease, lease_id)?;
        if !live(&lease, at) {
            return Ok(CaseReportEmailRun::Stale);
        }
        let authorized = self
            .store
            .authorize_delivery(&lease, at)
            .map_err(|_| unavailable("report email authorization failed"))?;
        let at = self.observed(at)?;
        let Some(message) = authorized else {
            return Ok(CaseReportEmailRun::Stale);
        };
        if message != lease.message {
            return Err(checks::inconsistent(
                "authorized report email differs from its persisted envelope",
            ));
        }
        if !live(&lease, at) {
            return Ok(CaseReportEmailRun::Stale);
        }
        let outcome = self
            .delivery
            .deliver(lease.delivery_id, &message)
            .unwrap_or(CaseReportEmailOutcome::Uncertain);
        self.record(&lease, outcome, self.observed(at)?)
    }

    fn observed(&self, previous: OffsetDateTime) -> Result<OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        checks::window(previous, at)?;
        Ok(at)
    }

    fn record(
        &self,
        lease: &CaseReportEmailLease,
        outcome: CaseReportEmailOutcome,
        at: OffsetDateTime,
    ) -> Result<CaseReportEmailRun, ApplicationError> {
        let state = match outcome {
            CaseReportEmailOutcome::Accepted => CaseReportEmailState::Accepted,
            CaseReportEmailOutcome::DefinitelyRejected => CaseReportEmailState::Rejected,
            CaseReportEmailOutcome::Uncertain => CaseReportEmailState::Uncertain,
        };
        let retry_at = if state == CaseReportEmailState::Uncertain {
            at.checked_add(Duration::seconds(RETRY_DELAY_SECONDS))
                .filter(|retry| *retry < lease.provider_deadline)
        } else {
            None
        };
        let recorded = self
            .store
            .finish_delivery(CaseReportEmailFinish {
                report_id: lease.report_id,
                delivery_id: lease.delivery_id,
                generation: lease.generation,
                lease_id: lease.lease_id,
                state,
                retry_at,
                at,
            })
            .map_err(|_| unavailable("report email outcome could not be recorded"))?;
        Ok(if recorded {
            CaseReportEmailRun::Recorded(state)
        } else {
            CaseReportEmailRun::Stale
        })
    }
}

fn validate_lease(lease: &CaseReportEmailLease, wanted: Uuid) -> Result<(), ApplicationError> {
    checks::time(lease.lease_until)?;
    checks::time(lease.provider_deadline)?;
    if lease.report_id.as_uuid().is_nil()
        || lease.delivery_id.is_nil()
        || lease.lease_id != wanted
        || lease.generation == 0
        || lease.generation > i64::MAX as u64
    {
        return Err(checks::inconsistent(
            "report email lease identity is invalid",
        ));
    }
    Ok(())
}

fn live(lease: &CaseReportEmailLease, at: OffsetDateTime) -> bool {
    at < lease.lease_until && at < lease.provider_deadline
}

fn unavailable(message: &str) -> ApplicationError {
    ApplicationError::Port(message.into())
}
