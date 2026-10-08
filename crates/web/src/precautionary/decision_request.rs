use super::{
    decision_effects::Outcome,
    decision_values::Values,
    measure_request::invalid,
    primitives::{digest, nullable, uuid},
    request::Context,
};
use crate::{error::ApiError, procedural_facts::object::Object};
use application::precautionary_measures::*;
use domain::{
    cases::CaseId,
    hearings::{HearingId, HearingRevision},
    precautionary_hearings::{PrecautionaryHearingId, PrecautionaryHearingRevision},
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Anchor {
    Initial {
        hearing_id: String,
        revision: u32,
        values_digest: String,
        submission_digest: String,
    },
    Precautionary {
        hearing_id: String,
        revision: u32,
        capture_digest: String,
    },
}
impl Anchor {
    fn validate(self) -> Result<MeasureDecisionAnchorRef, ApiError> {
        Ok(match self {
            Self::Initial {
                hearing_id,
                revision,
                values_digest,
                submission_digest,
            } => MeasureDecisionAnchorRef::Initial {
                hearing_id: HearingId::from_uuid(uuid(&hearing_id)?),
                revision: HearingRevision::new(revision).map_err(|_| invalid())?,
                values_digest: digest(&values_digest)?,
                submission_digest: digest(&submission_digest)?,
            },
            Self::Precautionary {
                hearing_id,
                revision,
                capture_digest,
            } => MeasureDecisionAnchorRef::Precautionary {
                hearing_id: PrecautionaryHearingId::from_uuid(uuid(&hearing_id)?),
                revision: PrecautionaryHearingRevision::new(revision).map_err(|_| invalid())?,
                capture_digest: digest(&capture_digest)?,
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    operation_id: String,
    decision_id: String,
    context: Object<Context>,
    values: Object<Values>,
    #[serde(deserialize_with = "nullable")]
    anchor: Option<Object<Anchor>>,
    outcome: Object<Outcome>,
}
impl Command {
    pub(super) fn validate(self, case: CaseId) -> Result<MeasureDecisionCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() {
            return Err(invalid());
        }
        Ok(MeasureDecisionCommand {
            operation_id: MeasureDecisionOperationId::from_uuid(uuid(&self.operation_id)?),
            decision_id: MeasureDecisionId::from_uuid(uuid(&self.decision_id)?),
            context: self.context.0.validate()?,
            values: self.values.0.validate()?,
            anchor: self.anchor.map(|v| v.0.validate()).transpose()?,
            outcome: self.outcome.0.validate()?,
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
    ) -> Result<(MeasureDecisionCommand, MeasureDecisionConfirmation), ApiError> {
        Ok((
            self.command.0.validate(case)?,
            MeasureDecisionConfirmation {
                submission_digest: digest(&self.expected_submission_digest)?,
                review_digest: digest(&self.expected_review_digest)?,
            },
        ))
    }
}
