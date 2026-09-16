use super::*;
use crate::ApplicationError;
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
/// Exact current base and immutable R1 scope; reading does not reserve an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudicialCalendarPreparation {
    pub calendar_id: JudicialCalendarId,
    pub base: Option<JudicialCalendarDetail>,
    pub initial_scope: Option<JudicialCalendarScope>,
}
pub trait JudicialCalendarStore: Send + Sync {
    fn list(
        &self,
        actor: UserId,
        query: JudicialCalendarQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarPage, ApplicationError>;
    /// Resolve the requested exact revision and verify its scope against R1.
    fn get(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        revision: Option<JudicialCalendarRevision>,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarDetail, ApplicationError>;
    fn history(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        query: JudicialCalendarHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError>;
    fn prepare(
        &self,
        actor: UserId,
        command: &JudicialCalendarCommand,
    ) -> Result<JudicialCalendarPreparation, ApplicationError>;
    /// Under the shared audit lock, revalidate active Owner, expected head, R1 scope,
    /// terminal status and operation uniqueness. Read Clock there and atomically
    /// insert revision and audit; failure must persist neither change.
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedJudicialCalendarChange,
    ) -> Result<JudicialCalendarDetail, ApplicationError>;
}
pub trait JudicialCalendarWorkflow: Send + Sync {
    fn list(
        &self,
        token: &str,
        query: JudicialCalendarQuery,
    ) -> Result<JudicialCalendarPage, ApplicationError>;
    fn get(
        &self,
        token: &str,
        id: JudicialCalendarId,
        revision: Option<JudicialCalendarRevision>,
    ) -> Result<JudicialCalendarDetail, ApplicationError>;
    fn history(
        &self,
        token: &str,
        id: JudicialCalendarId,
        query: JudicialCalendarHistoryQuery,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError>;
    fn days(
        &self,
        token: &str,
        id: JudicialCalendarId,
        revision: JudicialCalendarRevision,
        query: JudicialCalendarDaysQuery,
    ) -> Result<JudicialCalendarDays, ApplicationError>;
    fn prepare(
        &self,
        token: &str,
        command: JudicialCalendarCommand,
    ) -> Result<JudicialCalendarDraft, ApplicationError>;
    fn submit(
        &self,
        token: &str,
        command: JudicialCalendarCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<JudicialCalendarDetail, ApplicationError>;
}
