use crate::hearings::{
    HearingModality, HearingNote, HearingParticipantRef, HearingSupportRef, HearingTime,
    HearingVenue,
};
use crate::DomainError;

use super::ResourceHearingKind;

pub const MAX_RESOURCE_HEARING_PARTICIPANTS: usize = 32;

/// Declared scheduling basis with exact support; it is not judicial certification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingSchedulingBasis {
    statement: HearingNote,
    support: HearingSupportRef,
}

impl ResourceHearingSchedulingBasis {
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

/// Construction input; values validate the complete participant selection.
#[derive(Debug, Clone)]
pub struct ResourceHearingValuesInput {
    pub kind: ResourceHearingKind,
    pub scheduled_at: HearingTime,
    pub modality: HearingModality,
    pub venue: HearingVenue,
    pub note: Option<HearingNote>,
    pub participants: Vec<HearingParticipantRef>,
    pub scheduling_basis: ResourceHearingSchedulingBasis,
}

/// Immutable scheduling values, without stage, root identity or captured receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingValues {
    kind: ResourceHearingKind,
    scheduled_at: HearingTime,
    modality: HearingModality,
    venue: HearingVenue,
    note: Option<HearingNote>,
    participants: Vec<HearingParticipantRef>,
    scheduling_basis: ResourceHearingSchedulingBasis,
}

impl ResourceHearingValues {
    pub fn new(mut input: ResourceHearingValuesInput) -> Result<Self, DomainError> {
        if input.participants.len() > MAX_RESOURCE_HEARING_PARTICIPANTS {
            return Err(DomainError::InvalidHearingValue("participants"));
        }
        input
            .participants
            .sort_unstable_by_key(|reference| reference.id().as_uuid());
        if input
            .participants
            .windows(2)
            .any(|pair| pair[0].id() == pair[1].id())
        {
            return Err(DomainError::InvalidHearingValue("participants"));
        }
        Ok(Self {
            kind: input.kind,
            scheduled_at: input.scheduled_at,
            modality: input.modality,
            venue: input.venue,
            note: input.note,
            participants: input.participants,
            scheduling_basis: input.scheduling_basis,
        })
    }

    pub const fn kind(&self) -> ResourceHearingKind {
        self.kind
    }

    pub const fn scheduled_at(&self) -> HearingTime {
        self.scheduled_at
    }

    pub const fn modality(&self) -> HearingModality {
        self.modality
    }

    pub const fn venue(&self) -> &HearingVenue {
        &self.venue
    }

    pub const fn note(&self) -> Option<&HearingNote> {
        self.note.as_ref()
    }

    pub fn participants(&self) -> &[HearingParticipantRef] {
        &self.participants
    }

    pub const fn scheduling_basis(&self) -> &ResourceHearingSchedulingBasis {
        &self.scheduling_basis
    }
}
