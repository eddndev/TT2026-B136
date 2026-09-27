use crate::{
    deadlines::CommandInput,
    error::ApiError,
    procedural_facts::object::Object,
    resource_activities::{ActInput, ResourceInput},
};
use application::{
    deadlines::DeadlineChange, resource_activities::*, resource_deadlines::ResourceDeadlineCommand,
};
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    resource_id: String,
    association_id: String,
    expected_resource_revision: u32,
    resource: Object<ResourceInput>,
    #[serde(deserialize_with = "nullable")]
    act: Option<Object<ActInput>>,
    deadline: Object<CommandInput>,
}
fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Object<T>>, D::Error> {
    Option::<Object<T>>::deserialize(d)
}
impl Command {
    pub(super) fn validate(
        self,
        case: CaseId,
        resource: ResourceId,
    ) -> Result<ResourceDeadlineCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() || uuid(&self.resource_id)? != resource.as_uuid()
        {
            return Err(invalid());
        }
        let selected = self.resource.0.validate()?;
        if selected.id != resource {
            return Err(invalid());
        }
        let deadline = self.deadline.0.validate()?;
        let (command, _) = deadline.clone().into_parts();
        if !matches!(&command.change, DeadlineChange::Register { definition } if definition.input.selection.case_id == case)
        {
            return Err(invalid());
        }
        Ok(ResourceDeadlineCommand {
            association_id: ResourceActivityId::from_uuid(uuid(&self.association_id)?),
            expected_resource_revision: ResourceRevision::new(self.expected_resource_revision)
                .map_err(|_| invalid())?,
            resource: selected,
            act: self.act.map(|v| v.0.validate()).transpose()?,
            deadline,
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
    ) -> Result<(ResourceDeadlineCommand, Sha256Digest), ApiError> {
        let value = &self.expected_submission_digest;
        if value.len() != 64
            || !value
                .bytes()
                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
        {
            return Err(invalid());
        }
        let digest = Sha256Digest::from_hex(value).map_err(|_| invalid())?;
        Ok((self.command.0.validate(case, resource)?, digest))
    }
}
pub(super) fn uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| invalid())?;
    if id.to_string() != value {
        return Err(invalid());
    }
    Ok(id)
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_resource_deadline",
        "invalid contextual deadline command",
    )
}
