use crate::{error::ApiError, procedural_facts::object::Object};
use application::resource_activities::*;
use domain::{cases::CaseId, crypto::Sha256Digest, procedural_facts::FactText};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    resource_id: String,
    association_id: String,
    operation_id: String,
    expected_resource_revision: u32,
    change: Object<Change>,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Change {
    Link {
        expected_revision: u32,
        resource: Object<super::selection::Resource>,
        #[serde(deserialize_with = "nullable")]
        act: Option<Object<super::selection::Act>>,
        target: Object<super::selection::Target>,
    },
    Unlink {
        expected_revision: u32,
        reason: String,
    },
}
fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Object<T>>, D::Error> {
    Option::<Object<T>>::deserialize(d)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    command: Object<Command>,
    expected_submission_digest: String,
}
impl Command {
    pub(super) fn validate(
        self,
        case: CaseId,
        resource: ResourceId,
    ) -> Result<ResourceActivityCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() || uuid(&self.resource_id)? != resource.as_uuid()
        {
            return Err(invalid("route and command parents differ"));
        }
        let change = match self.change.0 {
            Change::Link {
                expected_revision,
                resource: selected,
                act,
                target,
            } => {
                if expected_revision != 0 {
                    return Err(invalid("link expects revision zero"));
                }
                let selection = ResourceActivitySelection {
                    resource: selected.0.validate()?,
                    act: act.map(|v| v.0.validate()).transpose()?,
                    target: target.0.validate()?,
                };
                if selection.resource.id != resource {
                    return Err(invalid("selected resource differs from route"));
                }
                ResourceActivityChange::Link { selection }
            }
            Change::Unlink {
                expected_revision,
                reason,
            } => ResourceActivityChange::Unlink {
                expected_revision: revision(expected_revision)?,
                reason: FactText::new(&reason).map_err(|_| invalid("invalid unlink reason"))?,
            },
        };
        let command = ResourceActivityCommand {
            association_id: ResourceActivityId::from_uuid(uuid(&self.association_id)?),
            operation_id: ResourceActivityOperationId::from_uuid(uuid(&self.operation_id)?),
            expected_resource_revision: ResourceRevision::new(self.expected_resource_revision)
                .map_err(|_| invalid("invalid resource revision"))?,
            change,
        };
        command
            .result_revision()
            .map_err(|_| invalid("association revision exhausted"))?;
        Ok(command)
    }
}
impl Submission {
    pub(super) fn validate(
        self,
        case: CaseId,
        resource: ResourceId,
    ) -> Result<(ResourceActivityCommand, Sha256Digest), ApiError> {
        Ok((
            self.command.0.validate(case, resource)?,
            digest(&self.expected_submission_digest)?,
        ))
    }
}
pub(super) fn uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    let id =
        uuid::Uuid::parse_str(value).map_err(|_| invalid("identifier must be a canonical UUID"))?;
    if id.to_string() != value {
        return Err(invalid("identifier must be a canonical UUID"));
    }
    Ok(id)
}
pub(super) fn digest(value: &str) -> Result<Sha256Digest, ApiError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("digest must be lowercase SHA-256 hexadecimal"));
    }
    Sha256Digest::from_hex(value).map_err(|_| invalid("invalid digest"))
}
pub(super) fn number(value: &str) -> Result<u32, ApiError> {
    if value.is_empty()
        || value.len() > 10
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid("number must be a positive canonical integer"));
    }
    value.parse().map_err(|_| invalid("number is outside u32"))
}
pub(super) fn revision(value: u32) -> Result<ResourceActivityRevision, ApiError> {
    ResourceActivityRevision::new(value).map_err(|_| invalid("invalid association revision"))
}
pub(super) fn invalid(message: &str) -> ApiError {
    ApiError::invalid_body("invalid_resource_activity", message)
}
