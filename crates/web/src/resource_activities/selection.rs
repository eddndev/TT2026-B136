use super::request::{digest, invalid, uuid};
use crate::error::ApiError;
use application::resource_activities::*;
use domain::{
    deadlines::{DeadlineId, DeadlineRevision},
    hearings::{HearingId, HearingRevision},
    procedural_resources::{ResourceActId, ResourceActRevision},
};
use serde::Deserialize;
use serde_json::{json, Value};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Resource {
    id: String,
    revision: u32,
    capture_digest: String,
}
impl Resource {
    pub(super) fn validate(self) -> Result<ResourceCaptureRef, ApiError> {
        Ok(ResourceCaptureRef {
            id: ResourceId::from_uuid(uuid(&self.id)?),
            revision: ResourceRevision::new(self.revision)
                .map_err(|_| invalid("invalid resource revision"))?,
            capture_digest: digest(&self.capture_digest)?,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Act {
    id: String,
    revision: u32,
    resource_revision: u32,
    capture_digest: String,
}
impl Act {
    pub(super) fn validate(self) -> Result<ResourceActCaptureRef, ApiError> {
        Ok(ResourceActCaptureRef {
            id: ResourceActId::from_uuid(uuid(&self.id)?),
            revision: ResourceActRevision::new(self.revision)
                .map_err(|_| invalid("invalid act revision"))?,
            resource_revision: ResourceRevision::new(self.resource_revision)
                .map_err(|_| invalid("invalid containing resource revision"))?,
            capture_digest: digest(&self.capture_digest)?,
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Target {
    Hearing {
        id: String,
        revision: u32,
        submission_digest: String,
    },
    Deadline {
        id: String,
        revision: u32,
        capture_digest: String,
    },
}
impl Target {
    pub(super) fn validate(self) -> Result<ResourceActivityTarget, ApiError> {
        Ok(match self {
            Self::Hearing {
                id,
                revision,
                submission_digest,
            } => ResourceActivityTarget::Hearing {
                id: HearingId::from_uuid(uuid(&id)?),
                revision: HearingRevision::new(revision)
                    .map_err(|_| invalid("invalid hearing revision"))?,
                submission_digest: digest(&submission_digest)?,
            },
            Self::Deadline {
                id,
                revision,
                capture_digest,
            } => ResourceActivityTarget::Deadline {
                id: DeadlineId::from_uuid(uuid(&id)?),
                revision: DeadlineRevision::new(revision)
                    .map_err(|_| invalid("invalid deadline revision"))?,
                capture_digest: digest(&capture_digest)?,
            },
        })
    }
}
pub(super) fn resource(v: ResourceCaptureRef) -> Value {
    json!({"id":v.id.to_string(),"revision":v.revision.get(),"capture_digest":v.capture_digest.to_hex()})
}
pub(super) fn project(v: ResourceActivitySelection) -> Value {
    let act = v.act.map(|a| json!({"id":a.id.to_string(),"revision":a.revision.get(),"resource_revision":a.resource_revision.get(),"capture_digest":a.capture_digest.to_hex()}));
    let target = match v.target {
        ResourceActivityTarget::Hearing {
            id,
            revision,
            submission_digest,
        } => {
            json!({"kind":"hearing","id":id.to_string(),"revision":revision.get(),"submission_digest":submission_digest.to_hex()})
        }
        ResourceActivityTarget::Deadline {
            id,
            revision,
            capture_digest,
        } => {
            json!({"kind":"deadline","id":id.to_string(),"revision":revision.get(),"capture_digest":capture_digest.to_hex()})
        }
    };
    json!({"resource":resource(v.resource),"act":act,"target":target})
}
pub(super) fn command(
    c: &ResourceActivityCommand,
    case: domain::cases::CaseId,
    resource: ResourceId,
) -> Value {
    let mut change = match &c.change {
        ResourceActivityChange::Link { selection } => project(*selection),
        ResourceActivityChange::Unlink { .. } => json!({"reason":c.reason().map(|r|r.as_str())}),
    };
    change["action"] = json!(c.action().as_str());
    change["expected_revision"] = json!(c.expected_revision());
    json!({"case_id":case,"resource_id":resource.to_string(),"association_id":c.association_id.to_string(),"operation_id":c.operation_id.to_string(),"expected_resource_revision":c.expected_resource_revision.get(),"change":change})
}
