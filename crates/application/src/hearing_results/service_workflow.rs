use super::service::support_error;
use super::validation_receipt::inconsistent;
use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::Sha256Digest, hearings::HearingId, identity::Permission};
impl HearingResultWorkflow for HearingResultService {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        hearing_id: HearingId,
        query: HearingResultQuery,
    ) -> Result<HearingResultPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearingResult)?;
        let page = self
            .store
            .list(actor, case_id, hearing_id, query, self.clock.now())?;
        if page
            .results
            .iter()
            .any(|row| row.case_id != case_id || row.hearing_id != hearing_id)
        {
            return Err(inconsistent("list contains a foreign hearing"));
        }
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        hearing_id: HearingId,
        id: HearingResultId,
        revision: Option<HearingResultRevision>,
    ) -> Result<HearingResultDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearingResult)?;
        let detail = self
            .store
            .get(actor, case_id, hearing_id, id, revision, self.clock.now())?;
        if detail.snapshot.case_id != case_id
            || detail.snapshot.hearing_id != hearing_id
            || detail.snapshot.id != id
            || revision.is_some_and(|v| v != detail.snapshot.revision)
        {
            return Err(inconsistent("detail differs from requested exact session"));
        }
        hearing_result_receipt_matches(self.hasher.as_ref(), &detail)?;
        Ok(detail)
    }
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        hearing_id: HearingId,
        id: HearingResultId,
        query: HearingResultHistoryQuery,
    ) -> Result<HearingResultHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadHearingResult)?;
        let page = self
            .store
            .history(actor, case_id, hearing_id, id, query, self.clock.now())?;
        for row in &page.revisions {
            if row.case_id != case_id || row.hearing_id != hearing_id || row.id != id {
                return Err(inconsistent("history contains a foreign session"));
            }
            hearing_result_history_receipt_matches(self.hasher.as_ref(), row)?;
        }
        Ok(page)
    }
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingResultCommand,
    ) -> Result<HearingResultDraft, ApplicationError> {
        self.prepare_command(token, case_id, command)
            .map_err(support_error)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: HearingResultCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<HearingResultDetail, ApplicationError> {
        self.submit_command(token, case_id, command, expected_submission_digest)
            .map_err(support_error)
    }
}
