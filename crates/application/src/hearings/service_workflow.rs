use super::service::support_error;
use super::validation::validate_context;
use super::validation_receipt::inconsistent;
use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::Sha256Digest, identity::Permission};

impl HearingWorkflow for HearingService {
    fn context(
        &self,
        token: &str,
        case_id: CaseId,
    ) -> Result<HearingCaseContext, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearing)?;
        let context = self.store.context(actor, case_id, self.clock.now())?;
        validate_context(self.hasher.as_ref(), case_id, &context)?;
        Ok(context)
    }
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: HearingQuery,
    ) -> Result<HearingPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearing)?;
        let page = self.store.list(actor, case_id, query, self.clock.now())?;
        if page.hearings.iter().any(|row| row.case_id != case_id) {
            return Err(inconsistent("list contains a foreign case"));
        }
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: HearingId,
        revision: Option<HearingRevision>,
    ) -> Result<HearingDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearing)?;
        let detail = self
            .store
            .get(actor, case_id, id, revision, self.clock.now())?;
        if detail.snapshot.case_id != case_id
            || detail.snapshot.id != id
            || revision.is_some_and(|revision| revision != detail.snapshot.revision)
        {
            return Err(inconsistent(
                "detail differs from the requested exact hearing",
            ));
        }
        hearing_receipt_matches(self.hasher.as_ref(), &detail)?;
        Ok(detail)
    }
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: HearingId,
        query: HearingHistoryQuery,
    ) -> Result<HearingHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearing)?;
        let page = self
            .store
            .history(actor, case_id, id, query, self.clock.now())?;
        for detail in &page.revisions {
            if detail.snapshot.case_id != case_id || detail.snapshot.id != id {
                return Err(inconsistent("history contains a foreign hearing"));
            }
            hearing_receipt_matches(self.hasher.as_ref(), detail)?;
        }
        Ok(page)
    }
    fn agenda(
        &self,
        token: &str,
        query: HearingAgendaQuery,
    ) -> Result<HearingAgendaPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearing)?;
        self.store.agenda(actor, query, self.clock.now())
    }
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingCommand,
    ) -> Result<HearingDraft, ApplicationError> {
        self.prepare_command(token, case_id, command)
            .map_err(support_error)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<HearingDetail, ApplicationError> {
        self.submit_command(token, case_id, command, expected_submission_digest)
            .map_err(support_error)
    }
}
