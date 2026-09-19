use super::*;
use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, crypto::Sha256Digest};
use std::sync::Mutex;

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub commands: Mutex<Vec<ResourceActivityCommand>>,
}
impl Workflow {
    fn call(&self, token: &str, value: Value) -> Result<(), ApplicationError> {
        let writing = matches!(value["method"].as_str(), Some("prepare" | "submit"));
        self.calls.lock().unwrap().push(value);
        if writing && token == "paralegal" {
            return Err(ApplicationError::PermissionDenied);
        }
        Err(match token {
            "client" | "revoked" => ApplicationError::PermissionDenied,
            "expired" => ApplicationError::InvalidSession,
            "closed" => ApplicationError::CaseClosed,
            "missing" => ResourceActivityError::NotFound.into(),
            "conflict" => ResourceActivityError::RevisionConflict.into(),
            "resource_conflict" => ResourceActivityError::ResourceRevisionConflict.into(),
            "operation" => ResourceActivityError::OperationConflict.into(),
            "mismatch" => ResourceActivityError::SubmissionMismatch.into(),
            "internal" => {
                ResourceActivityError::StoredInconsistent("private database detail".into()).into()
            }
            _ => return Ok(()),
        })
    }
    fn view(&self, token: &str, command: &ResourceActivityCommand) -> ResourceActivityView {
        let kind = if token == "deadline" {
            ResourceActivityKind::Deadline
        } else {
            ResourceActivityKind::Hearing
        };
        let mut command = command.clone();
        if matches!(token, "act" | "wrong_act_capture") {
            if let ResourceActivityChange::Link { selection } = &mut command.change {
                selection.act = Some(super::act::reference());
            }
        }
        let mut view = model::view(&command, kind);
        let row = &mut view.association;
        match token {
            "wrong_act_capture" => {
                row.sources.act.as_mut().unwrap().receipt.capture_digest =
                    Sha256Digest::from_array([9; 32]);
            }
            "foreign_case" => row.case_id = CaseId::from_uuid(FOREIGN.parse().unwrap()),
            "foreign_resource" => row.resource_id = ResourceId::from_uuid(FOREIGN.parse().unwrap()),
            "foreign_association" => {
                row.id = ResourceActivityId::from_uuid(FOREIGN.parse().unwrap())
            }
            "wrong_revision" => row.revision = ResourceActivityRevision::new(9).unwrap(),
            "wrong_target" => {
                if let ResourceActivityTargetDetail::Hearing(target) = &mut row.sources.target {
                    target.snapshot.id =
                        application::hearings::HearingId::from_uuid(FOREIGN.parse().unwrap());
                }
            }
            "wrong_current" => {
                if let ResourceActivityCurrentTarget::Hearing(target) = &mut view.current_target {
                    target.snapshot.id =
                        application::hearings::HearingId::from_uuid(FOREIGN.parse().unwrap());
                }
            }
            _ => {}
        }
        view
    }
}
impl ResourceActivityWorkflow for Workflow {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
    ) -> Result<ResourceActivityPage, ApplicationError> {
        self.call(token, json!({"method":"list","case_id":case,"resource_id":resource.to_string(),"limit":query.limit(),"after_id":query.after_id().map(|v|v.to_string())}))?;
        Ok(ResourceActivityPage {
            associations: vec![self.view(token, &command(ResourceActivityKind::Hearing, false))],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
    ) -> Result<ResourceActivityView, ApplicationError> {
        self.call(token, json!({"method":"get","case_id":case,"resource_id":resource.to_string(),"id":id.to_string(),"revision":revision.map(|v|v.get())}))?;
        let c = command(
            ResourceActivityKind::Hearing,
            revision.is_some_and(|v| v.get() == 2),
        );
        Ok(self.view(token, &c))
    }
    fn history(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError> {
        self.call(token, json!({"method":"history","case_id":case,"resource_id":resource.to_string(),"id":id.to_string(),"limit":query.limit(),"before_revision":query.before_revision().map(|v|v.get())}))?;
        Ok(ResourceActivityHistoryPage {
            revisions: vec![detail(
                &command(ResourceActivityKind::Hearing, false),
                ResourceActivityKind::Hearing,
            )],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
    ) -> Result<ResourceActivityDraft, ApplicationError> {
        self.call(
            token,
            json!({"method":"prepare","case_id":case,"resource_id":resource.to_string()}),
        )?;
        self.commands.lock().unwrap().push(command.clone());
        let kind = match &command.change {
            ResourceActivityChange::Link {
                selection:
                    ResourceActivitySelection {
                        target: ResourceActivityTarget::Deadline { .. },
                        ..
                    },
            } => ResourceActivityKind::Deadline,
            _ => ResourceActivityKind::Hearing,
        };
        Ok(draft(command, kind))
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        self.call(token, json!({"method":"submit","case_id":case,"resource_id":resource.to_string(),"digest":expected.to_hex()}))?;
        self.commands.lock().unwrap().push(command.clone());
        let kind = match &command.change {
            ResourceActivityChange::Link {
                selection:
                    ResourceActivitySelection {
                        target: ResourceActivityTarget::Deadline { .. },
                        ..
                    },
            } => ResourceActivityKind::Deadline,
            _ => ResourceActivityKind::Hearing,
        };
        Ok(detail(&command, kind))
    }
}
