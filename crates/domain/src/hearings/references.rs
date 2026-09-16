use crate::crypto::{DocumentVersionRef, Sha256Digest};
use crate::participants::{ParticipantId, ParticipantRevision};

use super::HearingNote;

/// Exact directory values selected for an appointment; it does not prove attendance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingParticipantRef {
    id: ParticipantId,
    revision: ParticipantRevision,
}

impl HearingParticipantRef {
    pub const fn new(id: ParticipantId, revision: ParticipantRevision) -> Self {
        Self { id, revision }
    }

    pub const fn id(self) -> ParticipantId {
        self.id
    }

    pub const fn revision(self) -> ParticipantRevision {
        self.revision
    }
}

/// Exact content supplied as support; later document versions are not substituted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingSupportRef {
    reference: DocumentVersionRef,
    digest: Sha256Digest,
}

impl HearingSupportRef {
    pub const fn new(reference: DocumentVersionRef, digest: Sha256Digest) -> Self {
        Self { reference, digest }
    }

    pub const fn reference(self) -> DocumentVersionRef {
        self.reference
    }

    pub const fn digest(self) -> Sha256Digest {
        self.digest
    }
}

/// Operator's declaration of a prior conviction, not a judicial certification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingConvictionBasis {
    statement: HearingNote,
    support: HearingSupportRef,
}

impl HearingConvictionBasis {
    pub const fn new(statement: HearingNote, support: HearingSupportRef) -> Self {
        Self { statement, support }
    }

    pub const fn statement(&self) -> &HearingNote {
        &self.statement
    }

    pub const fn support(&self) -> HearingSupportRef {
        self.support
    }
}
