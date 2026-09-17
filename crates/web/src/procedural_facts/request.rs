use super::{object::Object, values};
use crate::error::ApiError;
use application::{procedural_facts::*, ApplicationError};
use domain::{crypto::Sha256Digest, DomainError};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct Command(Object<Family>);
#[derive(Deserialize)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
enum Family {
    Resolution {
        operation_id: String,
        id: String,
        change: Box<Object<Change<values::Resolution>>>,
    },
    Notification {
        operation_id: String,
        id: String,
        resolution_id: String,
        change: Box<Object<Change<values::Notification>>>,
    },
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change<V> {
    Record {
        expected_revision: u32,
        values: Object<V>,
    },
    Correct {
        expected_revision: u32,
        values: Object<V>,
        reason: String,
    },
    Withdraw {
        expected_revision: u32,
        reason: String,
    },
}
impl<V> Change<V> {
    fn validate<T>(
        self,
        parse: impl FnOnce(V) -> Result<T, ApiError>,
    ) -> Result<FactChange<T>, ApiError> {
        let change = match self {
            Self::Record {
                expected_revision,
                values,
            } => {
                if expected_revision != 0 {
                    return Err(ApplicationError::from(DomainError::InvalidProceduralFact(
                        "expected_revision",
                    ))
                    .into());
                }
                FactChange::record(parse(values.0)?)
            }
            Self::Correct {
                expected_revision,
                values,
                reason,
            } => FactChange::correct(
                revision(expected_revision)?,
                parse(values.0)?,
                FactText::new(&reason).map_err(ApplicationError::from)?,
            ),
            Self::Withdraw {
                expected_revision,
                reason,
            } => FactChange::withdraw(
                revision(expected_revision)?,
                FactText::new(&reason).map_err(ApplicationError::from)?,
            ),
        };
        change.result_revision().map_err(ApplicationError::from)?;
        Ok(change)
    }
}
impl Command {
    pub(super) fn validate(self) -> Result<ProceduralFactCommand, ApiError> {
        match self.0 .0 {
            Family::Resolution {
                operation_id,
                id,
                change,
            } => Ok(ProceduralFactCommand::Resolution(ResolutionCommand::new(
                FactOperationId::from_uuid(parse_uuid(&operation_id, "invalid_fact_operation_id")?),
                ResolutionId::from_uuid(parse_uuid(&id, "invalid_resolution_id")?),
                (*change).0.validate(values::Resolution::validate)?,
            ))),
            Family::Notification {
                operation_id,
                id,
                resolution_id,
                change,
            } => Ok(ProceduralFactCommand::Notification(
                NotificationCommand::new(
                    FactOperationId::from_uuid(parse_uuid(
                        &operation_id,
                        "invalid_fact_operation_id",
                    )?),
                    NotificationId::from_uuid(parse_uuid(&id, "invalid_notification_id")?),
                    ResolutionId::from_uuid(parse_uuid(&resolution_id, "invalid_resolution_id")?),
                    (*change).0.validate(values::Notification::validate)?,
                )
                .map_err(ApplicationError::from)?,
            )),
        }
    }
}
#[derive(Deserialize)]
pub(super) struct Submission(Object<SubmissionFields>);
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmissionFields {
    command: Command,
    expected_submission_digest: String,
}
impl Submission {
    pub(super) fn validate(self) -> Result<(ProceduralFactCommand, Sha256Digest), ApiError> {
        let value = self.0 .0;
        Ok((
            value.command.validate()?,
            parse_digest(&value.expected_submission_digest)?,
        ))
    }
}
pub(super) fn revision(value: u32) -> Result<FactRevision, ApiError> {
    FactRevision::new(value).map_err(|e| ApplicationError::from(e).into())
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
            "invalid_procedural_fact_digest",
            "digest must contain 64 hexadecimal characters",
        )
    })
}
pub(super) fn project(command: &ProceduralFactCommand) -> Result<Value, ApiError> {
    let (mut body, change) = match command {
        ProceduralFactCommand::Resolution(v) => (
            json!({"family":"resolution","id":v.resolution_id().to_string()}),
            project_change(v.change(), values::resolution)?,
        ),
        ProceduralFactCommand::Notification(v) => (
            json!({"family":"notification","id":v.notification_id().to_string(),
                "resolution_id":v.resolution_id().to_string()}),
            project_change(v.change(), values::notification)?,
        ),
    };
    body["operation_id"] = json!(command.operation_id().to_string());
    body["change"] = change;
    Ok(body)
}
fn project_change<V>(
    change: &FactChange<V>,
    project: impl FnOnce(&V) -> Result<Value, ApiError>,
) -> Result<Value, ApiError> {
    Ok(match change {
        FactChange::Record { values } => {
            json!({"action":"record","expected_revision":0,"values":project(values)?})
        }
        FactChange::Correct {
            expected_revision,
            values,
            reason,
        } => json!({"action":"correct",
            "expected_revision":expected_revision.get(),"values":project(values)?,"reason":reason.as_str()}),
        FactChange::Withdraw {
            expected_revision,
            reason,
        } => json!({"action":"withdraw",
            "expected_revision":expected_revision.get(),"reason":reason.as_str()}),
    })
}
