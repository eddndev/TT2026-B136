//! Exact organizational links; linking never mutates an independent activity.
mod identity;
use crate::{
    crypto::Sha256Digest,
    deadlines::{DeadlineId, DeadlineRevision},
    hearings::{HearingId, HearingRevision},
    procedural_resources::{ResourceActId, ResourceActRevision, ResourceId, ResourceRevision},
};
pub use identity::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceActivityKind {
    Hearing,
    Deadline,
}
impl ResourceActivityKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hearing => "hearing",
            Self::Deadline => "deadline",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceActivityStatus {
    Linked,
    Unlinked,
}
impl ResourceActivityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linked => "linked",
            Self::Unlinked => "unlinked",
        }
    }
    pub const fn tag(self) -> u8 {
        match self {
            Self::Linked => 0,
            Self::Unlinked => 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCaptureRef {
    pub id: ResourceId,
    pub revision: ResourceRevision,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActCaptureRef {
    pub id: ResourceActId,
    pub revision: ResourceActRevision,
    pub resource_revision: ResourceRevision,
    pub capture_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceActivityTarget {
    Hearing {
        id: HearingId,
        revision: HearingRevision,
        submission_digest: Sha256Digest,
    },
    Deadline {
        id: DeadlineId,
        revision: DeadlineRevision,
        capture_digest: Sha256Digest,
    },
}
impl ResourceActivityTarget {
    pub const fn kind(self) -> ResourceActivityKind {
        match self {
            Self::Hearing { .. } => ResourceActivityKind::Hearing,
            Self::Deadline { .. } => ResourceActivityKind::Deadline,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceActivitySelection {
    pub resource: ResourceCaptureRef,
    pub act: Option<ResourceActCaptureRef>,
    pub target: ResourceActivityTarget,
}
impl ResourceActivitySelection {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"RASL1".to_vec();
        bytes.extend_from_slice(self.resource.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&self.resource.revision.get().to_be_bytes());
        bytes.extend_from_slice(self.resource.capture_digest.as_bytes());
        bytes.push(u8::from(self.act.is_some()));
        if let Some(act) = self.act {
            bytes.extend_from_slice(act.id.as_uuid().as_bytes());
            bytes.extend_from_slice(&act.revision.get().to_be_bytes());
            bytes.extend_from_slice(&act.resource_revision.get().to_be_bytes());
            bytes.extend_from_slice(act.capture_digest.as_bytes());
        }
        let (tag, id, revision, digest) = match self.target {
            ResourceActivityTarget::Hearing {
                id,
                revision,
                submission_digest,
            } => (0, id.as_uuid(), revision.get(), submission_digest),
            ResourceActivityTarget::Deadline {
                id,
                revision,
                capture_digest,
            } => (1, id.as_uuid(), revision.get(), capture_digest),
        };
        bytes.push(tag);
        bytes.extend_from_slice(id.as_bytes());
        bytes.extend_from_slice(&revision.to_be_bytes());
        bytes.extend_from_slice(digest.as_bytes());
        bytes
    }
}
