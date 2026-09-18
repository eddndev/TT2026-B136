use super::{
    service_validation::{validate_history, validate_page},
    *,
};
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::Sha256Digest, identity::Permission};

impl DeadlineWorkflow for DeadlineService {
    fn responsibles(
        &self,
        token: &str,
        case_id: CaseId,
        query: DeadlineResponsibleQuery,
    ) -> Result<DeadlineResponsiblePage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadline)?;
        let page = self
            .store
            .responsibles(actor.0, case_id, query, self.clock.now())?;
        page.validate(case_id, query)?;
        self.same_actor(token, actor, Permission::ReadDeadline)?;
        Ok(page)
    }
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: DeadlineQuery,
    ) -> Result<DeadlinePage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadline)?;
        let page = self
            .store
            .list(actor.0, case_id, query.clone(), self.clock.now())?;
        validate_page(case_id, &page, &query)?;
        self.same_actor(token, actor, Permission::ReadDeadline)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: DeadlineId,
        revision: Option<DeadlineRevision>,
    ) -> Result<DeadlineDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadline)?;
        let detail = self
            .store
            .get(actor.0, case_id, id, revision, self.clock.now())?;
        deadline_receipt_matches(self.hasher.as_ref(), &detail)?;
        if detail.case_id != case_id
            || detail.id != id
            || revision.is_some_and(|value| value != detail.revision)
        {
            return Err(inconsistent(
                "deadline detail differs from requested case, identity or exact revision",
            ));
        }
        self.same_actor(token, actor, Permission::ReadDeadline)?;
        Ok(detail)
    }
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DeadlineId,
        query: DeadlineHistoryQuery,
    ) -> Result<DeadlineHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadDeadline)?;
        let page = self
            .store
            .history(actor.0, case_id, id, query, self.clock.now())?;
        validate_history(self.hasher.as_ref(), case_id, id, &page, query)?;
        self.same_actor(token, actor, Permission::ReadDeadline)?;
        Ok(page)
    }
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: DeadlineCommand,
    ) -> Result<DeadlineDraft, ApplicationError> {
        self.prepare_command(token, case_id, command)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: DeadlineCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<DeadlineDetail, ApplicationError> {
        self.submit_command(token, case_id, command, expected_submission_digest)
    }
}
