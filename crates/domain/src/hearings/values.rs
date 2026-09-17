use crate::DomainError;

use super::{
    HearingConvictionBasis, HearingKind, HearingModality, HearingNote, HearingParticipantRef,
    HearingTime, HearingVenue,
};

pub const MAX_HEARING_PARTICIPANTS: usize = 32;

/// Construction input; only HearingValues guarantees cross-field invariants.
#[derive(Debug, Clone)]
pub struct HearingValuesInput {
    pub kind: HearingKind,
    pub scheduled_at: HearingTime,
    pub modality: HearingModality,
    pub venue: HearingVenue,
    pub note: Option<HearingNote>,
    pub participants: Vec<HearingParticipantRef>,
    pub conviction_basis: Option<HearingConvictionBasis>,
}

/// Validated appointment values independent of status, revision and capture metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingValues {
    kind: HearingKind,
    scheduled_at: HearingTime,
    modality: HearingModality,
    venue: HearingVenue,
    note: Option<HearingNote>,
    participants: Vec<HearingParticipantRef>,
    conviction_basis: Option<HearingConvictionBasis>,
}

impl HearingValues {
    pub fn new(mut input: HearingValuesInput) -> Result<Self, DomainError> {
        if input.participants.len() > MAX_HEARING_PARTICIPANTS {
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
        if (input.kind == HearingKind::Sentencing) != input.conviction_basis.is_some() {
            return Err(DomainError::InvalidHearingValue("conviction_basis"));
        }
        Ok(Self {
            kind: input.kind,
            scheduled_at: input.scheduled_at,
            modality: input.modality,
            venue: input.venue,
            note: input.note,
            participants: input.participants,
            conviction_basis: input.conviction_basis,
        })
    }

    pub const fn kind(&self) -> HearingKind {
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

    pub const fn conviction_basis(&self) -> Option<&HearingConvictionBasis> {
        self.conviction_basis.as_ref()
    }
}
