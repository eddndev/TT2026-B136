//! E-mail delivery of contract notices: one leased attempt per notice with a
//! durable outcome. The persisted sender, recipient, subject and text are
//! sent as stored, through the same provider port as ready-report e-mail;
//! no notice carries a token, signature or certificate. Signing never waits
//! for this consumer. Rationale: `docs/adr/0086-bilateral-contract-signing.md`.

use std::sync::Arc;

use domain::clock::{Clock, OffsetDateTime};
use domain::document_signing::Uuid;
use time::Duration;

use crate::{
    case_reports::{CaseReportEmailDelivery, CaseReportEmailMessage, CaseReportEmailOutcome},
    ApplicationError,
};

pub const NOTICE_LEASE_SECONDS: u32 = 30;
pub const NOTICE_PROVIDER_WINDOW_SECONDS: u32 = 86_400;
const RETRY_DELAY_SECONDS: i64 = 60;
/// Most notices one delivery cycle attempts.
pub const NOTICE_BATCH: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractNoticeClaim {
    pub lease_id: Uuid,
    pub lease_seconds: u32,
    pub provider_window_seconds: u32,
}

/// A claimed notice with its persisted envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractNoticeLease {
    pub id: Uuid,
    pub generation: u64,
    pub lease_id: Uuid,
    pub lease_until: OffsetDateTime,
    pub provider_deadline: OffsetDateTime,
    pub message: CaseReportEmailMessage,
}

/// Sent records provider acceptance, not arrival in the recipient's inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractNoticeState {
    Sent,
    Rejected,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractNoticeFinish {
    pub id: Uuid,
    pub generation: u64,
    pub lease_id: Uuid,
    pub state: ContractNoticeState,
    pub retry_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractNoticeRun {
    Idle,
    Stale,
    Recorded(ContractNoticeState),
}

/// Changes only the delivery columns of a notice, using the storage clock.
pub trait ContractNoticeStore: Send + Sync {
    /// Claims at most one pending notice, or an uncertain one whose retry
    /// time has come, of an active recipient and before its provider
    /// deadline, which the first claim fixes.
    fn claim_notice(
        &self,
        claim: ContractNoticeClaim,
    ) -> Result<Option<ContractNoticeLease>, ApplicationError>;
    /// Records the outcome when the exact lease is still live; a stale
    /// lease returns false. Only Uncertain schedules a retry.
    fn finish_notice(&self, finish: ContractNoticeFinish) -> Result<bool, ApplicationError>;
}

pub struct ContractNoticeWorker {
    store: Arc<dyn ContractNoticeStore>,
    delivery: Arc<dyn CaseReportEmailDelivery>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl ContractNoticeWorker {
    pub fn new(
        store: Arc<dyn ContractNoticeStore>,
        delivery: Arc<dyn CaseReportEmailDelivery>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            delivery,
            clock,
        }
    }

    /// Sends at most one notice once, within its lease.
    pub fn run_once(&self) -> Result<ContractNoticeRun, ApplicationError> {
        let lease_id = Uuid::new_v4();
        let lease = self
            .store
            .claim_notice(ContractNoticeClaim {
                lease_id,
                lease_seconds: NOTICE_LEASE_SECONDS,
                provider_window_seconds: NOTICE_PROVIDER_WINDOW_SECONDS,
            })
            .map_err(|_| unavailable("contract notice claim failed"))?;
        let Some(lease) = lease else {
            return Ok(ContractNoticeRun::Idle);
        };
        if lease.lease_id != lease_id || !live(&lease, self.clock.now()) {
            return Ok(ContractNoticeRun::Stale);
        }
        let outcome = self
            .delivery
            .deliver(lease.id, &lease.message)
            .unwrap_or(CaseReportEmailOutcome::Uncertain);
        let state = match outcome {
            CaseReportEmailOutcome::Accepted => ContractNoticeState::Sent,
            CaseReportEmailOutcome::DefinitelyRejected => ContractNoticeState::Rejected,
            CaseReportEmailOutcome::Uncertain => ContractNoticeState::Uncertain,
        };
        let retry_at = (state == ContractNoticeState::Uncertain)
            .then(|| {
                self.clock
                    .now()
                    .checked_add(Duration::seconds(RETRY_DELAY_SECONDS))
            })
            .flatten()
            .filter(|retry| *retry < lease.provider_deadline);
        let recorded = self
            .store
            .finish_notice(ContractNoticeFinish {
                id: lease.id,
                generation: lease.generation,
                lease_id: lease.lease_id,
                state,
                retry_at,
            })
            .map_err(|_| unavailable("contract notice outcome could not be recorded"))?;
        Ok(if recorded {
            ContractNoticeRun::Recorded(state)
        } else {
            ContractNoticeRun::Stale
        })
    }
}

fn live(lease: &ContractNoticeLease, at: OffsetDateTime) -> bool {
    at < lease.lease_until && at < lease.provider_deadline
}

fn unavailable(message: &str) -> ApplicationError {
    ApplicationError::Port(message.into())
}

#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
