use super::values::Values;
use crate::error::ApiError;
use application::{hearings::*, ApplicationError};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    crypto::Sha256Digest,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    operation_id: String,
    hearing_id: String,
    #[serde(deserialize_with = "super::object::deserialize")]
    change: Change,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Schedule {
        expected_revision: u32,
        expected_case_revision: u32,
        expected_stage_revision: u32,
        #[serde(deserialize_with = "super::object::deserialize")]
        values: Values,
    },
    Replace {
        expected_revision: u32,
        expected_case_revision: u32,
        expected_stage_revision: u32,
        #[serde(deserialize_with = "super::object::deserialize")]
        values: Values,
        reason: String,
    },
    Cancel {
        expected_revision: u32,
        reason: String,
    },
}
impl Command {
    pub fn validate(self) -> Result<HearingCommand, ApiError> {
        let operation_id = HearingOperationId::from_uuid(parse_uuid(
            &self.operation_id,
            "invalid_hearing_operation_id",
        )?);
        let hearing_id = HearingId::from_uuid(parse_uuid(&self.hearing_id, "invalid_hearing_id")?);
        let change = match self.change {
            Change::Schedule {
                expected_revision,
                expected_case_revision,
                expected_stage_revision,
                values,
            } => {
                if expected_revision != 0 {
                    return Err(ApplicationError::Domain(
                        domain::DomainError::InvalidHearingRevision,
                    )
                    .into());
                }
                HearingChange::Schedule {
                    context: context(expected_case_revision, expected_stage_revision)?,
                    values: values.validate()?,
                }
            }
            Change::Replace {
                expected_revision,
                expected_case_revision,
                expected_stage_revision,
                values,
                reason,
            } => HearingChange::Replace {
                expected_revision: revision(expected_revision)?,
                context: context(expected_case_revision, expected_stage_revision)?,
                values: values.validate()?,
                reason: HearingNote::new(&reason).map_err(ApplicationError::from)?,
            },
            Change::Cancel {
                expected_revision,
                reason,
            } => HearingChange::Cancel {
                expected_revision: revision(expected_revision)?,
                reason: HearingNote::new(&reason).map_err(ApplicationError::from)?,
            },
        };
        Ok(HearingCommand {
            operation_id,
            hearing_id,
            change,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    #[serde(deserialize_with = "super::object::deserialize")]
    command: Command,
    expected_submission_digest: String,
}
impl Submission {
    pub fn validate(self) -> Result<(HearingCommand, Sha256Digest), ApiError> {
        Ok((
            self.command.validate()?,
            parse_digest(&self.expected_submission_digest)?,
        ))
    }
}
fn context(case: u32, stage: u32) -> Result<HearingContextExpectation, ApiError> {
    Ok(HearingContextExpectation {
        case_revision: CaseRevision::new(case).map_err(ApplicationError::from)?,
        stage_revision: CaseStageRevision::new(stage).map_err(ApplicationError::from)?,
    })
}
pub(super) fn revision(value: u32) -> Result<HearingRevision, ApiError> {
    HearingRevision::new(value).map_err(|e| ApplicationError::from(e).into())
}
pub(super) fn parse_uuid(value: &str, code: &'static str) -> Result<Uuid, ApiError> {
    if value.len() > 36 {
        return Err(ApiError::invalid_body(code, "identifier must be a UUID"));
    }
    Uuid::parse_str(value).map_err(|_| ApiError::invalid_body(code, "identifier must be a UUID"))
}
pub(super) fn parse_digest(value: &str) -> Result<Sha256Digest, ApiError> {
    Sha256Digest::from_hex(value).map_err(|_| {
        ApiError::invalid_body(
            "invalid_hearing_digest",
            "digest must contain 64 hexadecimal characters",
        )
    })
}
