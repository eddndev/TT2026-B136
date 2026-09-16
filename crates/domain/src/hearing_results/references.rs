use super::{
    HearingResultAgreementId, HearingResultCapacity, HearingResultId, HearingResultObservation,
    HearingResultProvenanceKind, HearingResultReference, HearingResultRevision, HearingResultText,
};
use crate::crypto::{DocumentVersionRef, Sha256Digest};
use crate::participants::{ParticipantId, ParticipantRevision};
use crate::DomainError;

/// Exact historical directory entry and the reported capacity in this session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultAttendee {
    participant_id: ParticipantId,
    revision: ParticipantRevision,
    capacity: HearingResultCapacity,
    observation: Option<HearingResultObservation>,
}
impl HearingResultAttendee {
    pub const fn new(
        participant_id: ParticipantId,
        revision: ParticipantRevision,
        capacity: HearingResultCapacity,
        observation: Option<HearingResultObservation>,
    ) -> Self {
        Self {
            participant_id,
            revision,
            capacity,
            observation,
        }
    }
    pub const fn participant_id(&self) -> ParticipantId {
        self.participant_id
    }
    pub const fn revision(&self) -> ParticipantRevision {
        self.revision
    }
    pub const fn capacity(&self) -> &HearingResultCapacity {
        &self.capacity
    }
    pub const fn observation(&self) -> Option<&HearingResultObservation> {
        self.observation.as_ref()
    }
}

/// A declared item whose identity survives corrections independently of its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultAgreement {
    id: HearingResultAgreementId,
    text: HearingResultText,
}
impl HearingResultAgreement {
    pub const fn new(id: HearingResultAgreementId, text: HearingResultText) -> Self {
        Self { id, text }
    }
    pub const fn id(&self) -> HearingResultAgreementId {
        self.id
    }
    pub const fn text(&self) -> &HearingResultText {
        &self.text
    }
}

/// Exact content reference; document format and case scope are validated by ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultSupportRef {
    reference: DocumentVersionRef,
    digest: Sha256Digest,
}
impl HearingResultSupportRef {
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

/// A historical result revision; root existence and acyclicity require persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HearingResultContinuationRef {
    id: HearingResultId,
    revision: HearingResultRevision,
}
impl HearingResultContinuationRef {
    pub const fn new(id: HearingResultId, revision: HearingResultRevision) -> Self {
        Self { id, revision }
    }
    pub const fn id(self) -> HearingResultId {
        self.id
    }
    pub const fn revision(self) -> HearingResultRevision {
        self.revision
    }
}

/// Declared source class and locator, independent of optional documentary support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultProvenance {
    kind: HearingResultProvenanceKind,
    reference: Option<HearingResultReference>,
    support: Option<HearingResultSupportRef>,
}
impl HearingResultProvenance {
    pub fn new(
        kind: HearingResultProvenanceKind,
        reference: Option<HearingResultReference>,
        support: Option<HearingResultSupportRef>,
    ) -> Result<Self, DomainError> {
        if kind != HearingResultProvenanceKind::OperatorNote && reference.is_none() {
            return Err(DomainError::InvalidHearingResultValue(
                "provenance.reference",
            ));
        }
        Ok(Self {
            kind,
            reference,
            support,
        })
    }
    pub const fn kind(&self) -> HearingResultProvenanceKind {
        self.kind
    }
    pub const fn reference(&self) -> Option<&HearingResultReference> {
        self.reference.as_ref()
    }
    pub const fn support(&self) -> Option<HearingResultSupportRef> {
        self.support
    }
}
