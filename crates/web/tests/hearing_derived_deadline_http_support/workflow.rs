use application::{
    deadlines::DeadlineError, hearing_derived_deadlines::*, hearing_results::HearingResultError,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::Sha256Digest};
use std::sync::Mutex;

type RecordedCall = (
    String,
    CaseId,
    HearingDerivedDeadlineCommand,
    Option<Sha256Digest>,
);

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<RecordedCall>>,
    pub failure: Option<&'static str>,
    pub ready: Option<HearingDerivedDeadlineDraft>,
    pub replay: Option<HearingDerivedDeadlineRecord>,
}
impl Workflow {
    fn record(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
        expected: Option<Sha256Digest>,
    ) -> Result<(), ApplicationError> {
        self.calls
            .lock()
            .unwrap()
            .push((token.into(), case, command, expected));
        match self.failure {
            Some("session") => Err(ApplicationError::InvalidSession),
            Some("permission") => Err(ApplicationError::PermissionDenied),
            Some("case") => Err(ApplicationError::CaseNotFound),
            Some("closed") => Err(ApplicationError::CaseClosed),
            Some("source") => Err(HearingResultError::ReferenceNotFound.into()),
            Some("operation") => Err(DeadlineError::OperationConflict.into()),
            Some("mismatch") => Err(DeadlineError::SubmissionMismatch.into()),
            Some("invalid") => Err(DeadlineError::Invalid("declared source").into()),
            Some("stored") => {
                Err(DeadlineError::StoredInconsistent("secret SQL and source".into()).into())
            }
            Some("port") => Err(ApplicationError::Port("secret password and SQL".into())),
            _ => Ok(()),
        }
    }
}
impl HearingDerivedDeadlineWorkflow for Workflow {
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
    ) -> Result<HearingDerivedDeadlineReview, ApplicationError> {
        self.record(token, case, command, None)?;
        if let Some(record) = &self.replay {
            return Ok(HearingDerivedDeadlineReview::Replay(Box::new(
                record.clone(),
            )));
        }
        if let Some(draft) = &self.ready {
            return Ok(HearingDerivedDeadlineReview::Ready(Box::new(draft.clone())));
        }
        Err(ApplicationError::InvalidInput(
            "parsed command probe".into(),
        ))
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        command: HearingDerivedDeadlineCommand,
        expected: Sha256Digest,
    ) -> Result<HearingDerivedDeadlineRecord, ApplicationError> {
        self.record(token, case, command, Some(expected))?;
        self.replay
            .clone()
            .ok_or_else(|| ApplicationError::InvalidInput("parsed submission probe".into()))
    }
}
