use super::{FactLabel, FactProvenance, FactText};
use crate::participants::{ParticipantId, ParticipantRevision};

/// Selection of a historical directory entry; its subject is resolved by the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactParticipantRef {
    pub id: ParticipantId,
    pub revision: ParticipantRevision,
}

/// An unlinked declaration is not a verified directory identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactPerson {
    Participant(FactParticipantRef),
    Unlinked {
        label: FactLabel,
        description: FactText,
    },
}

/// Explicit endpoints and scope do not establish legal authority to represent someone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactRepresentation {
    NotRecorded(FactText),
    Declared {
        represented: FactPerson,
        representative: FactPerson,
        scope: FactText,
        provenance: Box<FactProvenance>,
    },
}
impl FactRepresentation {
    pub fn provenance(&self) -> Option<&FactProvenance> {
        match self {
            Self::NotRecorded(_) => None,
            Self::Declared { provenance, .. } => Some(provenance.as_ref()),
        }
    }
}
