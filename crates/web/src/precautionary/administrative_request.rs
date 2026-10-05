use super::{
    measure_request::{invalid, Reference, Subject},
    measure_time::Validity,
    primitives::{digest, uuid},
    request::Context,
};
use crate::{error::ApiError, procedural_facts::object::Object};
use application::measure_corrections::*;
use domain::{
    cases::CaseId,
    hearings::HearingNote,
    precautionary_hearings::MeasureId,
    precautionary_measures::{MeasureCorrectionOperationId, MeasureCorrectionValues},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Correction {
    conditions: String,
    validity: Object<Validity>,
    supervision_text: String,
}
impl Correction {
    fn validate(self) -> Result<MeasureCorrectionValues, ApiError> {
        Ok(MeasureCorrectionValues::new(
            HearingNote::new(&self.conditions).map_err(|_| invalid())?,
            self.validity.0.validate()?,
            HearingNote::new(&self.supervision_text).map_err(|_| invalid())?,
        ))
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Correct {
        values: Object<Correction>,
    },
    EnteredInError {},
    ReplaceEnteredInError {
        replacement_id: String,
        subject: Object<Subject>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    operation_id: String,
    target: Object<Reference>,
    context: Object<Context>,
    reason: String,
    action: Object<Action>,
}
impl Command {
    pub(super) fn validate(self, case: CaseId) -> Result<MeasureAdministrativeCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() {
            return Err(invalid());
        }
        let action = match self.action.0 {
            Action::Correct { values } => {
                MeasureAdministrativeAction::Correct(values.0.validate()?)
            }
            Action::EnteredInError {} => MeasureAdministrativeAction::MarkEnteredInError,
            Action::ReplaceEnteredInError {
                replacement_id,
                subject,
            } => MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
                replacement_id: MeasureId::from_uuid(uuid(&replacement_id)?),
                subject: subject.0.validate()?,
            },
        };
        Ok(MeasureAdministrativeCommand {
            operation_id: MeasureCorrectionOperationId::from_uuid(uuid(&self.operation_id)?),
            target: self.target.0.validate()?,
            context: self.context.0.validate()?,
            reason: HearingNote::new(&self.reason).map_err(|_| invalid())?,
            action,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    command: Object<Command>,
    expected_submission_digest: String,
    expected_review_digest: String,
}
impl Submission {
    pub(super) fn validate(
        self,
        case: CaseId,
    ) -> Result<
        (
            MeasureAdministrativeCommand,
            MeasureAdministrativeConfirmation,
        ),
        ApiError,
    > {
        Ok((
            self.command.0.validate(case)?,
            MeasureAdministrativeConfirmation {
                submission_digest: digest(&self.expected_submission_digest)?,
                review_digest: digest(&self.expected_review_digest)?,
            },
        ))
    }
}
