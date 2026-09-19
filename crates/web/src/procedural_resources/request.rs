use super::{
    object::Object,
    values::{text, ActValues, Values},
};
use crate::error::ApiError;
use application::{procedural_resources::*, ApplicationError};
use domain::{crypto::Sha256Digest, DomainError};
use serde::Deserialize;
use serde_json::{json, Value};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    operation_id: String,
    resource_id: String,
    change: Object<Change>,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Register {
        expected_revision: u32,
        values: Object<Values>,
    },
    Correct {
        expected_revision: u32,
        values: Object<Values>,
        reason: String,
    },
    RecordAct {
        expected_revision: u32,
        act_id: String,
        values: Object<ActValues>,
    },
    CorrectAct {
        expected_revision: u32,
        act_id: String,
        expected_act_revision: u32,
        values: Object<ActValues>,
        reason: String,
    },
    Archive {
        expected_revision: u32,
        reason: String,
    },
    Reactivate {
        expected_revision: u32,
        reason: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    command: Object<Command>,
    expected_submission_digest: String,
}
impl Command {
    pub(super) fn validate(self) -> Result<ResourceCommand, ApiError> {
        let change = match self.change.0 {
            Change::Register {
                expected_revision,
                values,
            } => {
                if expected_revision != 0 {
                    return Err(invalid("registration expects revision zero"));
                }
                ResourceChange::Register {
                    values: values.0.validate()?,
                }
            }
            Change::Correct {
                expected_revision,
                values,
                reason,
            } => ResourceChange::Correct {
                expected_revision: revision(expected_revision)?,
                values: values.0.validate()?,
                reason: text(reason)?,
            },
            Change::RecordAct {
                expected_revision,
                act_id,
                values,
            } => ResourceChange::RecordAct {
                expected_revision: revision(expected_revision)?,
                act_id: ResourceActId::from_uuid(uuid(&act_id)?),
                values: values.0.validate()?,
            },
            Change::CorrectAct {
                expected_revision,
                act_id,
                expected_act_revision,
                values,
                reason,
            } => ResourceChange::CorrectAct {
                expected_revision: revision(expected_revision)?,
                act_id: ResourceActId::from_uuid(uuid(&act_id)?),
                expected_act_revision: checked(ResourceActRevision::new(expected_act_revision))?,
                values: values.0.validate()?,
                reason: text(reason)?,
            },
            Change::Archive {
                expected_revision,
                reason,
            } => ResourceChange::Archive {
                expected_revision: revision(expected_revision)?,
                reason: text(reason)?,
            },
            Change::Reactivate {
                expected_revision,
                reason,
            } => ResourceChange::Reactivate {
                expected_revision: revision(expected_revision)?,
                reason: text(reason)?,
            },
        };
        let command = ResourceCommand {
            operation_id: ResourceOperationId::from_uuid(uuid(&self.operation_id)?),
            resource_id: ResourceId::from_uuid(uuid(&self.resource_id)?),
            change,
        };
        command.result_revision()?;
        Ok(command)
    }
}
impl Submission {
    pub(super) fn validate(self) -> Result<(ResourceCommand, Sha256Digest), ApiError> {
        let value = &self.expected_submission_digest;
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "submission digest must be lowercase SHA-256 hexadecimal",
            ));
        }
        Ok((
            self.command.0.validate()?,
            checked(Sha256Digest::from_hex(value))?,
        ))
    }
}
pub(super) fn checked<T>(v: Result<T, DomainError>) -> Result<T, ApiError> {
    v.map_err(|e| ApplicationError::from(e).into())
}
pub(super) fn uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    let id =
        uuid::Uuid::parse_str(value).map_err(|_| invalid("identifier must be a canonical UUID"))?;
    if id.to_string() != value {
        return Err(invalid("identifier must be a canonical UUID"));
    }
    Ok(id)
}
pub(super) fn revision(value: u32) -> Result<ResourceRevision, ApiError> {
    checked(ResourceRevision::new(value))
}
pub(super) fn invalid(message: &str) -> ApiError {
    ApiError::invalid_body("invalid_procedural_resource", message)
}
pub(super) fn action(value: ResourceAction) -> &'static str {
    match value {
        ResourceAction::Register => "register",
        ResourceAction::Correct => "correct",
        ResourceAction::RecordAct => "record_act",
        ResourceAction::CorrectAct => "correct_act",
        ResourceAction::Archive => "archive",
        ResourceAction::Reactivate => "reactivate",
    }
}
pub(super) fn project(c: &ResourceCommand) -> Result<Value, ApiError> {
    let mut change = json!({"action":action(c.action()),"expected_revision":c.expected_revision()});
    match &c.change {
        ResourceChange::Register { values } | ResourceChange::Correct { values, .. } => {
            change["values"] = super::values::values(values)?
        }
        ResourceChange::RecordAct { act_id, values, .. }
        | ResourceChange::CorrectAct { act_id, values, .. } => {
            change["act_id"] = json!(act_id.to_string());
            change["values"] = super::values::act(values)?;
        }
        _ => {}
    }
    if let ResourceChange::CorrectAct {
        expected_act_revision,
        ..
    } = &c.change
    {
        change["expected_act_revision"] = json!(expected_act_revision.get());
    }
    if let Some(reason) = c.reason() {
        change["reason"] = json!(reason.as_str());
    }
    Ok(
        json!({"operation_id":c.operation_id.to_string(),"resource_id":c.resource_id.to_string(),"change":change}),
    )
}
