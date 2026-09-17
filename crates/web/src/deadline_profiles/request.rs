use super::{definition::Definition, object};
use crate::error::ApiError;
use application::{deadline_profiles::*, ApplicationError};
use domain::{crypto::Sha256Digest, procedural_facts::FactText};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    operation_id: String,
    profile_id: String,
    #[serde(deserialize_with = "object::deserialize")]
    change: Change,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Publish {
        expected_revision: u32,
        #[serde(deserialize_with = "object::deserialize")]
        definition: Definition,
    },
    Replace {
        expected_revision: u32,
        #[serde(deserialize_with = "object::deserialize")]
        definition: Definition,
        reason: String,
    },
    Retire {
        expected_revision: u32,
        reason: String,
    },
}
impl Command {
    pub(super) fn validate(self) -> Result<DeadlineProfileCommand, ApiError> {
        let operation_id = DeadlineProfileOperationId::from_uuid(uuid(
            &self.operation_id,
            "invalid_deadline_profile_operation_id",
        )?);
        let profile_id =
            DeadlineProfileId::from_uuid(uuid(&self.profile_id, "invalid_deadline_profile_id")?);
        let change = match self.change {
            Change::Publish {
                expected_revision,
                definition,
            } => {
                if expected_revision != 0 {
                    return Err(super::definition::invalid());
                }
                DeadlineProfileChange::Publish {
                    definition: definition.validate()?,
                }
            }
            Change::Replace {
                expected_revision,
                definition,
                reason,
            } => DeadlineProfileChange::Replace {
                expected_revision: DeadlineProfileRevision::new(expected_revision)
                    .map_err(ApplicationError::from)?,
                definition: definition.validate()?,
                reason: FactText::new(&reason).map_err(ApplicationError::from)?,
            },
            Change::Retire {
                expected_revision,
                reason,
            } => DeadlineProfileChange::Retire {
                expected_revision: DeadlineProfileRevision::new(expected_revision)
                    .map_err(ApplicationError::from)?,
                reason: FactText::new(&reason).map_err(ApplicationError::from)?,
            },
        };
        Ok(DeadlineProfileCommand {
            operation_id,
            profile_id,
            change,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    #[serde(deserialize_with = "object::deserialize")]
    command: Command,
    expected_submission_digest: String,
}
impl Submission {
    pub(super) fn validate(self) -> Result<(DeadlineProfileCommand, Sha256Digest), ApiError> {
        let digest = Sha256Digest::from_hex(&self.expected_submission_digest).map_err(|_| {
            ApiError::invalid_body(
                "invalid_deadline_profile_digest",
                "digest must contain 64 hexadecimal characters",
            )
        })?;
        Ok((self.command.validate()?, digest))
    }
}
pub(super) fn uuid(v: &str, code: &'static str) -> Result<uuid::Uuid, ApiError> {
    if v.len() > 36 {
        return Err(ApiError::invalid_body(code, "identifier must be a UUID"));
    }
    uuid::Uuid::parse_str(v).map_err(|_| ApiError::invalid_body(code, "identifier must be a UUID"))
}
pub(super) fn parse_u32(v: &str) -> Option<u32> {
    if v.is_empty() || v.len() > 10 || !v.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        v.parse().ok()
    }
}
