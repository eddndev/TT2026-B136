use super::*;
use application::ApplicationError;
use std::sync::Mutex;

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub commands: Mutex<Vec<ProceduralFactCommand>>,
}
impl Workflow {
    fn record(&self, token: &str, call: Value) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(call);
        let error = match token {
            "capture" => ApplicationError::InvalidInput("captured input".into()),
            "forbidden" => ApplicationError::PermissionDenied,
            "expired" => ApplicationError::InvalidSession,
            "case_missing" => ApplicationError::CaseNotFound,
            "closed" => ApplicationError::CaseClosed,
            "missing" => ProceduralFactError::NotFound.into(),
            "source_missing" => ProceduralFactError::ReferenceNotFound.into(),
            "invalid_reference" => ProceduralFactError::InvalidReference.into(),
            "conflict" => ProceduralFactError::RevisionConflict.into(),
            "exhausted" => ProceduralFactError::RevisionExhausted.into(),
            "withdrawn" => ProceduralFactError::AlreadyWithdrawn.into(),
            "operation" => ProceduralFactError::OperationConflict.into(),
            "mismatch" => ProceduralFactError::SubmissionMismatch.into(),
            "changed" => ProceduralFactError::SupportChanged.into(),
            "too_large" => ProceduralFactError::SupportTooLarge.into(),
            "format" => ProceduralFactError::SupportFormatRejected.into(),
            "budget" => ProceduralFactError::SupportValidationLimit.into(),
            "digest" => ProceduralFactError::SupportDigestMismatch.into(),
            "internal" => {
                ProceduralFactError::StoredInconsistent("private SQL source".into()).into()
            }
            _ => return Ok(()),
        };
        Err(error)
    }
}
impl ProceduralFactWorkflow for Workflow {
    fn list_resolutions(
        &self,
        token: &str,
        scope: CaseId,
        q: ResolutionQuery,
    ) -> Result<ResolutionPage, ApplicationError> {
        self.record(token,json!({"method":"resolutions","case_id":scope.to_string(),"limit":q.limit(),"after_id":q.after_id().map(|i|i.to_string()),"status":q.status().status().map(status_name)}))?;
        let id = ResolutionId::from_uuid(Uuid::parse_str(ID).unwrap());
        let values = value_factory::resolution(&values_json("resolution"));
        Ok(ResolutionPage {
            resolutions: vec![ResolutionOverview {
                root: ResolutionRoot::new(id, scope),
                revision: FactRevision::initial(),
                status: q.status().status().unwrap_or(FactStatus::Recorded),
                class: values.class().clone(),
                issued_at: values.issued_at(),
            }],
            has_more: false,
            next_after_id: None,
        })
    }
    fn list_notifications(
        &self,
        token: &str,
        scope: CaseId,
        parent: ResolutionId,
        q: NotificationQuery,
    ) -> Result<NotificationPage, ApplicationError> {
        self.record(token,json!({"method":"notifications","case_id":scope.to_string(),"resolution_id":parent.to_string(),"limit":q.limit(),"after_id":q.after_id().map(|i|i.to_string()),"status":q.status().status().map(status_name)}))?;
        let id = NotificationId::from_uuid(Uuid::parse_str(ID).unwrap());
        let values = value_factory::notification(&values_json("notification"));
        Ok(NotificationPage {
            notifications: vec![NotificationOverview {
                root: NotificationRoot::new(id, scope, parent),
                revision: FactRevision::initial(),
                status: q.status().status().unwrap_or(FactStatus::Recorded),
                resolution: FactResolutionRef {
                    id: parent,
                    revision: FactRevision::new(3).unwrap(),
                },
                outcome: values.outcome().clone(),
                practiced_at: values.practiced_at(),
            }],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        token: &str,
        scope: CaseId,
        target: FactTarget,
        revision: Option<FactRevision>,
    ) -> Result<FactDetail, ApplicationError> {
        self.record(token,json!({"method":"get","case_id":scope.to_string(),"target":target_json(target),"revision":revision.map(|r|r.get())}))?;
        let mut result = detail(scope, &target_command(target));
        if let Some(revision) = revision {
            set_revision(&mut result, revision);
        }
        Ok(result)
    }
    fn history(
        &self,
        token: &str,
        scope: CaseId,
        target: FactTarget,
        q: FactHistoryQuery,
    ) -> Result<FactHistoryPage, ApplicationError> {
        self.record(token,json!({"method":"history","case_id":scope.to_string(),"target":target_json(target),"limit":q.limit(),"before_revision":q.before_revision().map(|r|r.get())}))?;
        Ok(FactHistoryPage {
            revisions: vec![FactHistoryEntry::from(
                &detail(scope, &target_command(target)).snapshot,
            )],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn prepare(
        &self,
        token: &str,
        scope: CaseId,
        command: ProceduralFactCommand,
    ) -> Result<FactDraft, ApplicationError> {
        self.commands.lock().unwrap().push(command.clone());
        self.record(token,json!({"method":"prepare","case_id":scope.to_string(),"target":target_json(command.target())}))?;
        let values = command_values(&command);
        let sources = sources(scope, &values);
        Ok(FactDraft {
            case_id: scope,
            actor: actor(),
            result_revision: command.result_revision()?,
            command,
            values,
            values_digest: digest(),
            sources,
            sources_digest: digest(),
            submission_digest: digest(),
            observed_administration: administration(),
        })
    }
    fn submit(
        &self,
        token: &str,
        scope: CaseId,
        command: ProceduralFactCommand,
        expected: Sha256Digest,
    ) -> Result<FactDetail, ApplicationError> {
        self.commands.lock().unwrap().push(command.clone());
        self.record(token,json!({"method":"submit","case_id":scope.to_string(),"target":target_json(command.target()),"expected_submission_digest":expected.to_hex()}))?;
        Ok(detail(scope, &command))
    }
}
