//! Durable ready-report email identity and one-attempt delivery ports.

use domain::{clock::OffsetDateTime, typed_participants::Uuid};

use super::CaseReportId;
use crate::ApplicationError;

/// Validated generic content for a first delivery claim. It contains no report
/// data or credentials; its sole link opens the authenticated reports entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportEmailTemplate {
    pub from: String,
    pub subject: String,
    pub text: String,
}

/// The complete envelope persisted before the first provider attempt. Retries
/// use these original fields even after sender or public-origin changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportEmailMessage {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportEmailOutcome {
    Accepted,
    DefinitelyRejected,
    Uncertain,
}

/// Accepted records provider acceptance, not delivery to the recipient's inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportEmailState {
    Pending,
    Accepted,
    Rejected,
    Uncertain,
    AccessRevoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportEmailRun {
    Idle,
    Stale,
    Recorded(CaseReportEmailState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportEmailClaim {
    pub lease_id: Uuid,
    pub at: OffsetDateTime,
    pub lease_seconds: u32,
    pub provider_window_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportEmailLease {
    pub report_id: CaseReportId,
    pub delivery_id: Uuid,
    pub generation: u64,
    pub lease_id: Uuid,
    pub lease_until: OffsetDateTime,
    pub provider_deadline: OffsetDateTime,
    pub message: CaseReportEmailMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportEmailFinish {
    pub report_id: CaseReportId,
    pub delivery_id: Uuid,
    pub generation: u64,
    pub lease_id: Uuid,
    pub state: CaseReportEmailState,
    pub retry_at: Option<OffsetDateTime>,
    pub at: OffsetDateTime,
}

/// Eligible intent is inserted with both artifacts, Ready, notice and audit.
/// Every claim and authorization rechecks the original active requester, its
/// authentication generation and complete captured case access under locks.
/// Mail transitions never change the report's availability or artifacts.
pub trait CaseReportEmailStore: Send + Sync {
    /// Claims at most one ready-report intent using the durable clock. Persist
    /// the recipient from the original requester and the first template before
    /// returning a lease. Later templates cannot replace this envelope. Retries
    /// preserve delivery_id and the first provider deadline; expiry fences the
    /// old worker. No row is claimed before its retry time or after its deadline.
    fn claim_delivery(
        &self,
        claim: CaseReportEmailClaim,
        template: &CaseReportEmailTemplate,
    ) -> Result<Option<CaseReportEmailLease>, ApplicationError>;

    /// Recheck authorization and the exact live lease immediately before I/O.
    /// Return the original persisted envelope. A stale claim returns None;
    /// revoked access also returns None after terminally stopping this delivery.
    fn authorize_delivery(
        &self,
        lease: &CaseReportEmailLease,
        at: OffsetDateTime,
    ) -> Result<Option<CaseReportEmailMessage>, ApplicationError>;

    /// Fence by report, delivery, generation, lease UUID and nonexpired lease.
    /// A stale result returns false. Only Uncertain can schedule a retry, strictly
    /// before the original provider deadline. Uncertain without retry is terminal.
    /// Store failure remains an error because the commit outcome may be unknown.
    fn finish_delivery(&self, result: CaseReportEmailFinish) -> Result<bool, ApplicationError>;
}

/// Exactly one bounded provider call. Its stable key is derived from delivery_id;
/// it sends the persisted message without rebuilding it from current settings.
/// No attachments, credentials, automatic HTTP retries or public download links.
pub trait CaseReportEmailDelivery: Send + Sync {
    fn deliver(
        &self,
        delivery_id: Uuid,
        message: &CaseReportEmailMessage,
    ) -> Result<CaseReportEmailOutcome, ApplicationError>;
}
