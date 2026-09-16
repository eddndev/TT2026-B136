use super::values::Values;
use crate::error::ApiError;
use application::{hearing_results::*, ApplicationError};
use domain::{
    crypto::Sha256Digest,
    hearings::{HearingId, HearingRevision},
};
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    operation_id: String,
    hearing_id: String,
    result_id: String,
    #[serde(deserialize_with = "super::object::deserialize")]
    change: Change,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Record {
        expected_revision: u32,
        anchor_revision: u32,
        #[serde(default, deserialize_with = "super::object::optional")]
        continuation: Option<Continuation>,
        #[serde(deserialize_with = "super::object::deserialize")]
        values: Values,
    },
    Correct {
        expected_revision: u32,
        #[serde(deserialize_with = "super::object::deserialize")]
        values: Values,
        reason: String,
    },
    Withdraw {
        expected_revision: u32,
        reason: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Continuation {
    result_id: String,
    revision: u32,
}
impl Command {
    pub fn validate(self) -> Result<HearingResultCommand, ApiError> {
        let operation_id = HearingResultOperationId::from_uuid(parse_uuid(
            &self.operation_id,
            "invalid_hearing_result_operation_id",
        )?);
        let hearing_id = HearingId::from_uuid(parse_uuid(&self.hearing_id, "invalid_hearing_id")?);
        let result_id =
            HearingResultId::from_uuid(parse_uuid(&self.result_id, "invalid_hearing_result_id")?);
        let change = match self.change {
            Change::Record {
                expected_revision,
                anchor_revision,
                continuation,
                values,
            } => {
                if expected_revision != 0 {
                    return Err(ApplicationError::Domain(
                        domain::DomainError::InvalidHearingResultRevision,
                    )
                    .into());
                }
                HearingResultChange::Record {
                    anchor_revision: HearingRevision::new(anchor_revision)
                        .map_err(ApplicationError::from)?,
                    continuation: continuation
                        .map(|c| {
                            Ok::<_, ApiError>(HearingResultContinuationRef::new(
                                HearingResultId::from_uuid(parse_uuid(
                                    &c.result_id,
                                    "invalid_hearing_result_id",
                                )?),
                                revision(c.revision)?,
                            ))
                        })
                        .transpose()?,
                    values: values.validate()?,
                }
            }
            Change::Correct {
                expected_revision,
                values,
                reason,
            } => HearingResultChange::Correct {
                expected_revision: revision(expected_revision)?,
                values: values.validate()?,
                reason: HearingResultText::new(&reason).map_err(ApplicationError::from)?,
            },
            Change::Withdraw {
                expected_revision,
                reason,
            } => HearingResultChange::Withdraw {
                expected_revision: revision(expected_revision)?,
                reason: HearingResultText::new(&reason).map_err(ApplicationError::from)?,
            },
        };
        Ok(HearingResultCommand {
            operation_id,
            hearing_id,
            result_id,
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
    pub fn validate(self) -> Result<(HearingResultCommand, Sha256Digest), ApiError> {
        Ok((
            self.command.validate()?,
            parse_digest(&self.expected_submission_digest)?,
        ))
    }
}
pub(super) fn revision(value: u32) -> Result<HearingResultRevision, ApiError> {
    HearingResultRevision::new(value).map_err(|e| ApplicationError::from(e).into())
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
            "invalid_hearing_result_digest",
            "digest must contain 64 hexadecimal characters",
        )
    })
}
