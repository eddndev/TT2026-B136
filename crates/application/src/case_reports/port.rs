use super::*;
use crate::{identity::Principal, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::Sha256Digest};

pub trait CaseReportWorkflow: Send + Sync {
    fn litigators(
        &self,
        token: &str,
        query: CaseReportLitigatorQuery,
    ) -> Result<CaseReportLitigatorPage, ApplicationError>;
    fn acknowledge_notice(
        &self,
        token: &str,
        id: CaseReportId,
    ) -> Result<CaseReportDetail, ApplicationError>;
    fn request(
        &self,
        token: &str,
        command: CaseReportCommand,
    ) -> Result<CaseReportDetail, ApplicationError>;
    fn list(&self, token: &str, query: CaseReportQuery)
        -> Result<CaseReportPage, ApplicationError>;
    fn get(&self, token: &str, id: CaseReportId) -> Result<CaseReportDetail, ApplicationError>;
    fn download(
        &self,
        token: &str,
        id: CaseReportId,
        format: CaseReportFormat,
    ) -> Result<CaseReportDownload, ApplicationError>;
}
/// Each operation reloads the durable principal and original account stamp under
/// the audit lock. A restored role cannot restore an old authorization generation.
/// Reads are restricted to the same requester and originally authorized scope.
/// Download validates every captured case again before decrypting either artifact.
pub trait CaseReportStore: Send + Sync {
    fn litigators(
        &self,
        actor: &Principal,
        query: CaseReportLitigatorQuery,
        at: OffsetDateTime,
    ) -> Result<CaseReportLitigatorPage, ApplicationError>;
    fn acknowledge_notice(
        &self,
        actor: &Principal,
        id: CaseReportId,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError>;
    fn request(
        &self,
        actor: &Principal,
        scope: CaseReportScope,
        command: CaseReportCommand,
        request_digest: Sha256Digest,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError>;
    fn list(
        &self,
        actor: &Principal,
        query: CaseReportQuery,
        at: OffsetDateTime,
    ) -> Result<CaseReportPage, ApplicationError>;
    fn get(
        &self,
        actor: &Principal,
        id: CaseReportId,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError>;
    fn download(
        &self,
        actor: &Principal,
        id: CaseReportId,
        format: CaseReportFormat,
        at: OffsetDateTime,
    ) -> Result<CaseReportDownload, ApplicationError>;
}
/// Rendering is bounded and outside DB transactions. Both formats consume the
/// same immutable capture; PDF requires Unicode layout and CSV neutralizes formulas.
pub trait CaseReportRenderer: Send + Sync {
    fn render(
        &self,
        snapshot: &CaseReportSnapshot,
        format: CaseReportFormat,
    ) -> Result<Vec<u8>, ApplicationError>;
}
/// Request, capture and completion revalidate active account stamp, original
/// scope and every case membership. A lost assignment revokes the whole report.
/// Claims are exclusive and expire. Every write compares report/attempt/token/
/// generation and a nonexpired lease using the durable clock. Snapshot is immutable.
/// Completion atomically commits both artifacts, Ready and one own-report notice.
/// Failure and retry transitions are also fenced; raw diagnostics are never public.
pub trait CaseReportWorkerStore: Send + Sync {
    fn claim_next(&self, at: OffsetDateTime) -> Result<Option<CaseReportClaim>, ApplicationError>;
    fn capture(
        &self,
        lease: &CaseReportLease,
        at: OffsetDateTime,
    ) -> Result<CaseReportSnapshot, ApplicationError>;
    fn renew(
        &self,
        lease: &CaseReportLease,
        at: OffsetDateTime,
    ) -> Result<CaseReportLease, ApplicationError>;
    fn complete(
        &self,
        lease: &CaseReportLease,
        snapshot: &CaseReportSnapshot,
        artifacts: Vec<CaseReportArtifact>,
        at: OffsetDateTime,
    ) -> Result<CaseReportDetail, ApplicationError>;
    fn fail(
        &self,
        lease: &CaseReportLease,
        failure: CaseReportFailure,
        at: OffsetDateTime,
    ) -> Result<CaseReportWorkerRun, ApplicationError>;
}
