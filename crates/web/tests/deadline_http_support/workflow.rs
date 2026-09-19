use application::deadline_currentness::DeadlineCurrent;
use application::{deadline_tracking::TrackingPolicies, deadlines::*, ApplicationError};
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};
use std::sync::Mutex;

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub command: Mutex<Option<DeadlineCommand>>,
    pub policies: Mutex<Option<TrackingPolicies>>,
    pub response: Mutex<Option<DeadlineDetail>>,
    pub current_response: Mutex<Option<DeadlineCurrent>>,
    pub responsible_page: Mutex<Option<DeadlineResponsiblePage>>,
    pub failure: Mutex<Option<&'static str>>,
    pub return_draft: bool,
}
impl Workflow {
    fn record(
        &self,
        token: &str,
        case: CaseId,
        call: Value,
        write: bool,
    ) -> Result<(), ApplicationError> {
        self.calls
            .lock()
            .unwrap()
            .push(json!([token, case.to_string(), call]));
        if token == "client" || write && !matches!(token, "owner" | "litigator") {
            return Err(ApplicationError::PermissionDenied);
        }
        match *self.failure.lock().unwrap() {
            Some("case") => Err(ApplicationError::CaseNotFound),
            Some("closed") => Err(ApplicationError::CaseClosed),
            Some("session") => Err(ApplicationError::InvalidSession),
            Some("conflict") => Err(DeadlineError::RevisionConflict.into()),
            Some("mismatch") => Err(DeadlineError::SubmissionMismatch.into()),
            Some("retired") => Err(DeadlineError::Retired.into()),
            Some("responsible") => Err(DeadlineError::ResponsibleUnavailable.into()),
            Some("profile") => Err(DeadlineError::ProfileUnavailable.into()),
            _ => Ok(()),
        }
    }
}
impl DeadlineWorkflow for Workflow {
    fn responsibles(
        &self,
        token: &str,
        case: CaseId,
        query: DeadlineResponsibleQuery,
    ) -> Result<DeadlineResponsiblePage, ApplicationError> {
        self.record(
            token,
            case,
            json!([
                "responsibles",
                query.limit(),
                query.after_id().map(|id| id.to_string())
            ]),
            false,
        )?;
        Ok(self
            .responsible_page
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(DeadlineResponsiblePage {
                case_id: case,
                responsibles: vec![],
                has_more: false,
                next_after_id: None,
            }))
    }
    fn list(
        &self,
        token: &str,
        case: CaseId,
        query: DeadlineQuery,
    ) -> Result<DeadlinePage, ApplicationError> {
        self.record(
            token,
            case,
            json!([
                "list",
                query.limit(),
                query.after_id().map(|v| v.to_string()),
                query.status().status().map(|v| v.as_str())
            ]),
            false,
        )?;
        Ok(DeadlinePage {
            deadlines: self
                .response
                .lock()
                .unwrap()
                .as_ref()
                .map(|detail| {
                    DeadlineCurrent::historical(&super::records::Hasher, detail)
                        .map(|current| DeadlineOverview::from(&current))
                })
                .transpose()?
                .into_iter()
                .collect(),
            has_more: false,
            next_after_id: None,
        })
    }
    fn current(
        &self,
        token: &str,
        case: CaseId,
        id: DeadlineId,
    ) -> Result<DeadlineCurrent, ApplicationError> {
        let detail = self.get(token, case, id, None)?;
        if let Some(current) = self.current_response.lock().unwrap().clone() {
            return Ok(current);
        }
        DeadlineCurrent::historical(&super::records::Hasher, &detail)
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        id: DeadlineId,
        revision: Option<DeadlineRevision>,
    ) -> Result<DeadlineDetail, ApplicationError> {
        self.record(
            token,
            case,
            json!(["get", id.to_string(), revision.map(|v| v.get())]),
            false,
        )?;
        self.response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| DeadlineError::NotFound.into())
    }
    fn history(
        &self,
        token: &str,
        case: CaseId,
        id: DeadlineId,
        query: DeadlineHistoryQuery,
    ) -> Result<DeadlineHistoryPage, ApplicationError> {
        self.record(
            token,
            case,
            json!([
                "history",
                id.to_string(),
                query.limit(),
                query.before_revision().map(|v| v.get())
            ]),
            false,
        )?;
        let guard = self.response.lock().unwrap();
        let detail = guard.as_ref().ok_or(DeadlineError::NotFound)?;
        Ok(DeadlineHistoryPage {
            revisions: vec![DeadlineHistoryEntry::from_detail(
                &super::records::Hasher,
                detail,
            )?],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        human: DeadlineHumanCommand,
    ) -> Result<DeadlineDraft, ApplicationError> {
        let (command, policies) = human.clone().into_parts();
        self.record(
            token,
            case,
            json!([
                "prepare",
                command.deadline_id.to_string(),
                command.action().as_str()
            ]),
            true,
        )?;
        *self.command.lock().unwrap() = Some(command.clone());
        *self.policies.lock().unwrap() = policies;
        if self.return_draft {
            let guard = self.response.lock().unwrap();
            return Ok(super::records::draft(
                human,
                guard.as_ref().expect("fixture detail"),
            ));
        }
        Err(ApplicationError::InvalidInput(
            "parsed command probe".into(),
        ))
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        human: DeadlineHumanCommand,
        digest: Sha256Digest,
    ) -> Result<DeadlineDetail, ApplicationError> {
        let (command, policies) = human.into_parts();
        self.record(
            token,
            case,
            json!([
                "submit",
                command.deadline_id.to_string(),
                command.action().as_str(),
                digest.to_hex()
            ]),
            true,
        )?;
        *self.command.lock().unwrap() = Some(command);
        *self.policies.lock().unwrap() = policies;
        self.response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| ApplicationError::InvalidInput("parsed submission probe".into()))
    }
}
