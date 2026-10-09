use super::*;
use crate::{cases::CaseStatusFilter, identity::Principal};
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision},
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::UserId,
};

pub const MAX_REPORT_CASES: usize = 1000;
pub const MAX_REPORT_ASSIGNMENTS: usize = 10000;
pub const MAX_REPORT_WORKLOAD: usize = 1000;
pub const MAX_REPORT_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_REPORT_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportScope {
    Office,
    AssignedCases,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportFormat {
    Pdf,
    Csv,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportKind {
    CaseState,
    LitigatorActivity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportFilters {
    pub kind: CaseReportKind,
    pub period_from: OffsetDateTime,
    pub period_before: OffsetDateTime,
    pub status: CaseStatusFilter,
    pub litigator: Option<UserId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportCommand {
    pub operation_id: CaseReportOperationId,
    pub filters: CaseReportFilters,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseReportQuery {
    pub limit: u32,
    pub after_id: Option<CaseReportId>,
    pub unread_only: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseReportLitigatorQuery {
    pub kind: CaseReportKind,
    pub limit: u32,
    pub after_id: Option<UserId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportRequester {
    pub principal: Principal,
    pub account_revision: u64,
    pub auth_generation: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportPhase {
    Capturing,
    Rendering,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportFailure {
    TemporaryUnavailable,
    RenderUnavailable,
    RenderFailed,
    CapacityExceeded,
    InvalidStoredCapture,
    AccessRevoked,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseReportState {
    Queued,
    Processing(CaseReportPhase),
    RetryWaiting {
        phase: CaseReportPhase,
        retry_at: OffsetDateTime,
    },
    Ready {
        snapshot_digest: Sha256Digest,
        checked_at: OffsetDateTime,
        artifacts: Vec<CaseReportArtifactMetadata>,
    },
    Failed(CaseReportFailure),
    AccessRevoked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseReportNoticeKind {
    Ready,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportNotice {
    pub kind: CaseReportNoticeKind,
    pub created_at: OffsetDateTime,
    pub read_at: Option<OffsetDateTime>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportDetail {
    pub id: CaseReportId,
    pub requester: CaseReportRequester,
    pub scope: CaseReportScope,
    pub command: CaseReportCommand,
    pub request_digest: Sha256Digest,
    pub requested_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub state: CaseReportState,
    pub notice: Option<CaseReportNotice>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportPage {
    pub checked_at: OffsetDateTime,
    pub reports: Vec<CaseReportDetail>,
    pub has_more: bool,
    pub next_after_id: Option<CaseReportId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportLitigator {
    pub user_id: UserId,
    pub email: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportLitigatorPage {
    pub scope: CaseReportScope,
    pub checked_at: OffsetDateTime,
    pub litigators: Vec<CaseReportLitigator>,
    pub has_more: bool,
    pub next_after_id: Option<UserId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportRow {
    pub case_id: CaseId,
    pub title: String,
    pub reference: String,
    pub created_at: OffsetDateTime,
    pub status: CaseAdministrativeStatus,
    pub administration_revision: Option<CaseRevision>,
    pub administration_digest: Option<Sha256Digest>,
    pub assigned_litigators: Vec<CaseReportLitigator>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportWorkload {
    pub litigator: CaseReportLitigator,
    pub active_cases: u64,
    pub closed_cases: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportActivityRow {
    pub case_id: CaseId,
    pub litigator_id: UserId,
    pub documents_uploaded: u64,
    pub procedural_activities: u64,
    pub deadlines_attended: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportActivitySnapshot {
    pub actors: Vec<CaseReportLitigator>,
    pub rows: Vec<CaseReportActivityRow>,
    pub documents_complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportSnapshot {
    pub report_id: CaseReportId,
    pub requester: CaseReportRequester,
    pub scope: CaseReportScope,
    pub filters: CaseReportFilters,
    pub checked_at: OffsetDateTime,
    pub cases: Vec<CaseReportRow>,
    pub workload: Vec<CaseReportWorkload>,
    pub activity: Option<CaseReportActivitySnapshot>,
    pub digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportArtifactMetadata {
    pub format: CaseReportFormat,
    pub bytes: u64,
    pub digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportArtifact {
    pub report_id: CaseReportId,
    pub requester: CaseReportRequester,
    pub scope: CaseReportScope,
    pub format: CaseReportFormat,
    pub snapshot_digest: Sha256Digest,
    pub digest: Sha256Digest,
    pub content: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportDownload {
    pub report: CaseReportDetail,
    pub artifact: CaseReportArtifact,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportLease {
    pub report_id: CaseReportId,
    pub attempt_id: CaseReportAttemptId,
    pub token: CaseReportLeaseToken,
    pub generation: u64,
    pub expires_at: OffsetDateTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReportClaim {
    pub lease: CaseReportLease,
    pub report: CaseReportDetail,
    pub snapshot: Option<CaseReportSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseReportWorkerRun {
    Idle,
    Ready(CaseReportId),
    Deferred(CaseReportId),
    Failed(CaseReportId),
    AccessRevoked(CaseReportId),
}
