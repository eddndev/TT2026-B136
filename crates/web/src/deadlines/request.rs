use super::{input::Definition, object::Object};
use crate::{error::ApiError, procedural_facts::values::time::DeclaredTime};
use application::{
    deadline_tracking::{TrackingPolicies, TrackingPolicy},
    deadlines::*,
    ApplicationError,
};
use domain::{
    crypto::Sha256Digest,
    procedural_facts::{FactLabel, FactText},
};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    operation_id: String,
    deadline_id: String,
    change: Object<Change>,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Register {
        expected_revision: u32,
        definition: Object<Definition>,
        tracking: Object<Policies>,
    },
    Correct {
        expected_revision: u32,
        definition: Object<Definition>,
        reason: String,
        tracking: Object<Policies>,
    },
    SetAttention {
        expected_revision: u32,
        attention: Object<Attention>,
        reason: String,
    },
    Retire {
        expected_revision: u32,
        reason: String,
    },
}
#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
enum Attention {
    Pending {},
    Recorded {
        occurred_at: Object<DeclaredTime>,
        statement: String,
        locator: String,
    },
}
impl Command {
    pub(super) fn validate(self) -> Result<DeadlineHumanCommand, ApiError> {
        let (change, policies) = match self.change.0 {
            Change::Register {
                expected_revision,
                definition,
                tracking,
            } => {
                if expected_revision != 0 {
                    return Err(invalid());
                }
                (
                    DeadlineChange::Register {
                        definition: definition.0.validate()?,
                    },
                    Some(tracking.0.value()),
                )
            }
            Change::Correct {
                expected_revision,
                definition,
                reason,
                tracking,
            } => (
                DeadlineChange::Correct {
                    expected_revision: checked(DeadlineRevision::new(expected_revision))?,
                    definition: definition.0.validate()?,
                    reason: checked(FactText::new(&reason))?,
                },
                Some(tracking.0.value()),
            ),
            Change::SetAttention {
                expected_revision,
                attention,
                reason,
            } => (
                DeadlineChange::SetAttention {
                    expected_revision: checked(DeadlineRevision::new(expected_revision))?,
                    attention: attention.0.validate()?,
                    reason: checked(FactText::new(&reason))?,
                },
                None,
            ),
            Change::Retire {
                expected_revision,
                reason,
            } => (
                DeadlineChange::Retire {
                    expected_revision: checked(DeadlineRevision::new(expected_revision))?,
                    reason: checked(FactText::new(&reason))?,
                },
                None,
            ),
        };
        checked(DeadlineHumanCommand::new(
            DeadlineCommand {
                operation_id: DeadlineOperationId::from_uuid(uuid(
                    &self.operation_id,
                    "invalid_deadline_operation_id",
                )?),
                deadline_id: DeadlineId::from_uuid(uuid(&self.deadline_id, "invalid_deadline_id")?),
                change,
            },
            policies,
        ))
    }
}
impl Attention {
    fn validate(self) -> Result<DeadlineAttention, ApiError> {
        Ok(match self {
            Self::Pending {} => DeadlineAttention::Pending,
            Self::Recorded {
                occurred_at,
                statement,
                locator,
            } => DeadlineAttention::Recorded {
                occurred_at: occurred_at.0.validate()?,
                statement: checked(FactText::new(&statement))?,
                locator: checked(FactLabel::new(&locator))?,
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    command: Object<Command>,
    expected_submission_digest: String,
}
impl Submission {
    pub(super) fn validate(self) -> Result<(DeadlineHumanCommand, Sha256Digest), ApiError> {
        let digest = Sha256Digest::from_hex(&self.expected_submission_digest).map_err(|_| {
            ApiError::invalid_body(
                "invalid_deadline_digest",
                "digest must contain 64 hexadecimal characters",
            )
        })?;
        Ok((self.command.0.validate()?, digest))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policies {
    profile: Policy,
    source: Policy,
    calendar: Policy,
}
impl Policies {
    fn value(self) -> TrackingPolicies {
        TrackingPolicies {
            profile: self.profile.value(),
            source: self.source.value(),
            calendar: self.calendar.value(),
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Policy {
    Fixed,
    Follow,
    Undetermined,
}
impl Policy {
    fn value(self) -> TrackingPolicy {
        match self {
            Self::Fixed => TrackingPolicy::Fixed,
            Self::Follow => TrackingPolicy::Follow,
            Self::Undetermined => TrackingPolicy::Undetermined,
        }
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
pub(super) fn checked<T, E: Into<ApplicationError>>(v: Result<T, E>) -> Result<T, ApiError> {
    v.map_err(|e| ApiError::from(e.into()))
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_deadline", "invalid deadline command field")
}
