use super::{authorization, port, preparation, storage, PostgresDeadlineStore};
use application::{deadline_currentness::DeadlineCurrent, deadlines::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};
impl DeadlineStore for PostgresDeadlineStore {
    fn responsibles(
        &self,
        actor: UserId,
        case: CaseId,
        query: DeadlineResponsibleQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineResponsiblePage, ApplicationError> {
        self.responsible_page(actor, case, query, at)
    }
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        query: DeadlineQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlinePage, ApplicationError> {
        self.list_page(actor, case, query, at)
    }
    fn current(
        &self,
        actor: UserId,
        case: CaseId,
        id: DeadlineId,
    ) -> Result<DeadlineCurrent, ApplicationError> {
        self.current_detail(actor, case, id)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        id: DeadlineId,
        revision: Option<DeadlineRevision>,
        at: OffsetDateTime,
    ) -> Result<DeadlineDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, case, false, self.hasher.as_ref())?;
        if !authorization::visible(&mut tx, case, id)? {
            return Err(DeadlineError::NotFound.into());
        }
        let detail = storage::detail(&mut tx, case, id, revision, self.hasher.as_ref())?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "deadline.read",
            &format!(
                "case:{case}:deadline:{id}:revision:{}:capture:{}",
                detail.revision.get(),
                detail.receipt.capture_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        id: DeadlineId,
        query: DeadlineHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineHistoryPage, ApplicationError> {
        self.history_page(actor, case, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &DeadlineCommand,
    ) -> Result<DeadlinePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        authorization::actor(&mut tx, actor, case, true, self.hasher.as_ref())?;
        let result = preparation::load(&mut tx, case, command, self.hasher.as_ref())?;
        tx.rollback().map_err(port)?;
        Ok(result)
    }
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineChange,
    ) -> Result<DeadlineDetail, ApplicationError> {
        self.commit_change(actor, prepared)
    }
}
