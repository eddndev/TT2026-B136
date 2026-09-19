use super::*;
use application::{procedural_resources::*, ApplicationError};
use domain::{cases::CaseId, crypto::Sha256Digest};
use std::sync::Mutex;
use uuid::Uuid;
#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub commands: Mutex<Vec<ResourceCommand>>,
}
impl Workflow {
    fn call(&self, token: &str, value: Value) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(value);
        Err(match token {
            "forbidden" => ApplicationError::PermissionDenied,
            "expired" => ApplicationError::InvalidSession,
            "closed" => ApplicationError::CaseClosed,
            "missing" => ProceduralResourceError::NotFound.into(),
            "conflict" => ProceduralResourceError::RevisionConflict.into(),
            "operation" => ProceduralResourceError::OperationConflict.into(),
            "archived" => ProceduralResourceError::Archived.into(),
            "unchanged" => ProceduralResourceError::StateUnchanged.into(),
            "mismatch" => ProceduralResourceError::SubmissionMismatch.into(),
            "internal" => {
                ProceduralResourceError::StoredInconsistent("private database detail".into()).into()
            }
            _ => return Ok(()),
        })
    }
    fn corrupt(&self, token: &str, mut row: ResourceDetail) -> ResourceDetail {
        match token {
            "foreign" => row.case_id = CaseId::new(),
            "wrong_revision" => row.revision = ResourceRevision::new(99).unwrap(),
            "wrong_source" => {
                row.sources.resolution.snapshot.reference.revision =
                    application::procedural_facts::FactRevision::initial()
            }
            "wrong_support" => row.sources.supports[0].digest = digest(),
            "unsafe_name" => row.sources.supports[0].name = "Unsafe support.pdf".into(),
            "wrong_previous" => row.receipt.previous = None,
            _ => {}
        }
        row
    }
}
impl ProceduralResourceWorkflow for Workflow {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        q: ResourceQuery,
    ) -> Result<ResourcePage, ApplicationError> {
        self.call(
            token,
            json!({"method":"list","case_id":case,"limit":q.limit(),
            "kind":q.kind().map(|v|v.as_str()),"status":q.status().map(|v|v.as_str()),
            "after_id":q.after_id().map(|v|v.to_string())}),
        )?;
        Ok(ResourcePage {
            resources: vec![self.corrupt(
                token,
                detail(
                    case,
                    &command(ResourceId::from_uuid(Uuid::parse_str(ID).unwrap()), 1),
                ),
            )],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        id: ResourceId,
        revision: Option<ResourceRevision>,
    ) -> Result<ResourceDetail, ApplicationError> {
        self.call(token,json!({"method":"get","case_id":case,"id":id.to_string(),"revision":revision.map(|r|r.get())}))?;
        Ok(self.corrupt(
            token,
            detail(case, &command(id, revision.map(|r| r.get()).unwrap_or(1))),
        ))
    }
    fn history(
        &self,
        token: &str,
        case: CaseId,
        id: ResourceId,
        q: ResourceHistoryQuery,
    ) -> Result<ResourceHistoryPage, ApplicationError> {
        self.call(token,json!({"method":"history","before_revision":q.before_revision().map(|r|r.get()),"limit":q.limit()}))?;
        Ok(ResourceHistoryPage {
            revisions: vec![self.corrupt(token, detail(case, &command(id, 1)))],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        command: ResourceCommand,
    ) -> Result<ResourceDraft, ApplicationError> {
        self.commands.lock().unwrap().push(command.clone());
        self.call(token, json!({"method":"prepare"}))?;
        let row = self.corrupt(token, detail(case, &command));
        Ok(ResourceDraft {
            case_id: row.case_id,
            result_revision: row.revision,
            command,
            values: row.values,
            status: row.status,
            sources: row.sources,
            act: row.act,
            previous: row.receipt.previous,
            recorded_by: row.recorded_by,
            observed_administration: row.recorded_administration,
            observed_stage: row.recorded_stage,
            submission_digest: digest(),
        })
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        command: ResourceCommand,
        expected: Sha256Digest,
    ) -> Result<ResourceDetail, ApplicationError> {
        self.commands.lock().unwrap().push(command.clone());
        self.call(token, json!({"method":"submit","digest":expected.to_hex()}))?;
        Ok(self.corrupt(token, detail(case, &command)))
    }
}
