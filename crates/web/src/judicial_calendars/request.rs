use super::values::Values;
use crate::error::ApiError;
use application::{judicial_calendars::*, ApplicationError};
use domain::crypto::Sha256Digest;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    operation_id: String,
    calendar_id: String,
    #[serde(deserialize_with = "super::object::deserialize")]
    change: Change,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Publish {
        expected_revision: u32,
        #[serde(deserialize_with = "super::object::deserialize")]
        values: Values,
    },
    Replace {
        expected_revision: u32,
        #[serde(deserialize_with = "super::object::deserialize")]
        values: Values,
        reason: String,
    },
    Retire {
        expected_revision: u32,
        reason: String,
    },
}
impl Command {
    pub fn validate(self) -> Result<JudicialCalendarCommand, ApiError> {
        let operation_id = JudicialCalendarOperationId::from_uuid(parse_uuid(
            &self.operation_id,
            "invalid_judicial_calendar_operation_id",
        )?);
        let calendar_id = super::parse_id(&self.calendar_id)?;
        let change = match self.change {
            Change::Publish {
                expected_revision,
                values,
            } => {
                if expected_revision != 0 {
                    return Err(ApplicationError::Domain(
                        domain::DomainError::InvalidJudicialCalendarRevision,
                    )
                    .into());
                }
                JudicialCalendarChange::Publish {
                    values: values.validate()?,
                }
            }
            Change::Replace {
                expected_revision,
                values,
                reason,
            } => JudicialCalendarChange::Replace {
                expected_revision: revision(expected_revision)?,
                values: values.validate()?,
                reason: JudicialCalendarReason::new(&reason).map_err(ApplicationError::from)?,
            },
            Change::Retire {
                expected_revision,
                reason,
            } => JudicialCalendarChange::Retire {
                expected_revision: revision(expected_revision)?,
                reason: JudicialCalendarReason::new(&reason).map_err(ApplicationError::from)?,
            },
        };
        Ok(JudicialCalendarCommand {
            operation_id,
            calendar_id,
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
    pub fn validate(self) -> Result<(JudicialCalendarCommand, Sha256Digest), ApiError> {
        let digest = Sha256Digest::from_hex(&self.expected_submission_digest).map_err(|_| {
            ApiError::invalid_body(
                "invalid_judicial_calendar_digest",
                "digest must contain 64 hexadecimal characters",
            )
        })?;
        Ok((self.command.validate()?, digest))
    }
}
pub(super) fn revision(value: u32) -> Result<JudicialCalendarRevision, ApiError> {
    JudicialCalendarRevision::new(value).map_err(|e| ApplicationError::from(e).into())
}
pub(super) fn parse_uuid(value: &str, code: &'static str) -> Result<Uuid, ApiError> {
    if value.len() > 36 {
        return Err(ApiError::invalid_body(code, "identifier must be a UUID"));
    }
    Uuid::parse_str(value).map_err(|_| ApiError::invalid_body(code, "identifier must be a UUID"))
}
pub(super) fn parse_u32(value: &str) -> Option<u32> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
