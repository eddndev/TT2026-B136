use super::{primitives::*, values::Values};
use crate::{error::ApiError, procedural_facts::object::Object};
use application::precautionary_hearings::*;
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    hearings::HearingNote,
    precautionary_hearings::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    administration_revision: u32,
    stage_revision: u32,
    context_digest: String,
}
impl Context {
    pub(super) fn validate(self) -> Result<PrecautionaryContextExpectation, ApiError> {
        Ok(PrecautionaryContextExpectation {
            administration_revision: CaseRevision::new(self.administration_revision)
                .map_err(|_| invalid())?,
            stage_revision: CaseStageRevision::new(self.stage_revision).map_err(|_| invalid())?,
            context_digest: digest(&self.context_digest)?,
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Schedule {
        context: Object<Context>,
        values: Object<Values>,
    },
    Replace {
        expected_revision: u32,
        expected_capture_digest: String,
        context: Object<Context>,
        values: Object<Values>,
        reason: String,
    },
    Cancel {
        expected_revision: u32,
        expected_capture_digest: String,
        reason: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    operation_id: String,
    hearing_id: String,
    change: Object<Change>,
}
impl Command {
    pub(super) fn validate(self, case: CaseId) -> Result<PrecautionaryHearingCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() {
            return Err(invalid());
        }
        let change = match self.change.0 {
            Change::Schedule { context, values } => PrecautionaryHearingChange::Schedule {
                context: context.0.validate()?,
                values: values.0.validate()?,
            },
            Change::Replace {
                expected_revision,
                expected_capture_digest,
                context,
                values,
                reason,
            } => PrecautionaryHearingChange::Replace {
                expected_revision: PrecautionaryHearingRevision::new(expected_revision)
                    .map_err(|_| invalid())?,
                expected_capture_digest: digest(&expected_capture_digest)?,
                context: context.0.validate()?,
                values: values.0.validate()?,
                reason: HearingNote::new(&reason).map_err(|_| invalid())?,
            },
            Change::Cancel {
                expected_revision,
                expected_capture_digest,
                reason,
            } => PrecautionaryHearingChange::Cancel {
                expected_revision: PrecautionaryHearingRevision::new(expected_revision)
                    .map_err(|_| invalid())?,
                expected_capture_digest: digest(&expected_capture_digest)?,
                reason: HearingNote::new(&reason).map_err(|_| invalid())?,
            },
        };
        let command = PrecautionaryHearingCommand {
            operation_id: PrecautionaryHearingOperationId::from_uuid(uuid(&self.operation_id)?),
            hearing_id: PrecautionaryHearingId::from_uuid(uuid(&self.hearing_id)?),
            change,
        };
        command.result_revision().map_err(|_| invalid())?;
        Ok(command)
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
            PrecautionaryHearingCommand,
            PrecautionaryHearingConfirmation,
        ),
        ApiError,
    > {
        Ok((
            self.command.0.validate(case)?,
            PrecautionaryHearingConfirmation {
                submission_digest: digest(&self.expected_submission_digest)?,
                review_digest: digest(&self.expected_review_digest)?,
            },
        ))
    }
}
