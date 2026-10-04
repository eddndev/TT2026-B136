use super::values::Values;
use crate::{
    error::ApiError,
    procedural_facts::object::Object,
    resource_activities::{ActInput, ResourceInput},
};
use application::{
    resource_activities::{ResourceActivityId, ResourceId, ResourceRevision},
    resource_hearings::ResourceHearingCommand,
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    resource_hearings::{ResourceHearingId, ResourceHearingOperationId},
};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    resource_id: String,
    operation_id: String,
    hearing_id: String,
    association_id: String,
    expected_resource_revision: u32,
    resource: Object<ResourceInput>,
    #[serde(deserialize_with = "nullable")]
    act: Option<Object<ActInput>>,
    values: Object<Values>,
}

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<Object<T>>, D::Error> {
    Option::<Object<T>>::deserialize(deserializer)
}

impl Command {
    pub(super) fn validate(
        self,
        case: CaseId,
        resource: ResourceId,
    ) -> Result<ResourceHearingCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() || uuid(&self.resource_id)? != resource.as_uuid()
        {
            return Err(invalid());
        }
        let selected = self.resource.0.validate()?;
        if selected.id != resource {
            return Err(invalid());
        }
        Ok(ResourceHearingCommand {
            operation_id: ResourceHearingOperationId::from_uuid(uuid(&self.operation_id)?),
            hearing_id: ResourceHearingId::from_uuid(uuid(&self.hearing_id)?),
            association_id: ResourceActivityId::from_uuid(uuid(&self.association_id)?),
            expected_resource_revision: ResourceRevision::new(self.expected_resource_revision)
                .map_err(|_| invalid())?,
            resource: selected,
            act: self.act.map(|value| value.0.validate()).transpose()?,
            values: self.values.0.validate()?,
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
    pub(super) fn validate(
        self,
        case: CaseId,
        resource: ResourceId,
    ) -> Result<(ResourceHearingCommand, Sha256Digest), ApiError> {
        Ok((
            self.command.0.validate(case, resource)?,
            digest(&self.expected_submission_digest)?,
        ))
    }
}

pub(super) fn uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| invalid())?;
    if id.to_string() != value {
        return Err(invalid());
    }
    Ok(id)
}

pub(super) fn digest(value: &str) -> Result<Sha256Digest, ApiError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid());
    }
    Sha256Digest::from_hex(value).map_err(|_| invalid())
}

pub(super) fn number(value: &str) -> Result<u32, ApiError> {
    if value.is_empty()
        || value.len() > 10
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    value.parse().map_err(|_| invalid())
}

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_resource_hearing",
        "invalid resource hearing command",
    )
}
