use crate::hearings::{
    HearingModality, HearingNote, HearingParticipantRef, HearingTime, HearingVenue,
};
use crate::DomainError;

use super::{
    PrecautionaryHearingPurpose, PrecautionaryHearingSchedulingBasis, PrecautionaryMeasureRef,
};

pub const MAX_PRECAUTIONARY_HEARING_PARTICIPANTS: usize = 32;
pub const MAX_PRECAUTIONARY_REVIEW_TARGETS: usize = 32;

/// Construction input; the complete selection is checked without truncation.
#[derive(Debug, Clone)]
pub struct PrecautionaryHearingValuesInput {
    pub purpose: PrecautionaryHearingPurpose,
    pub scheduled_at: HearingTime,
    pub modality: HearingModality,
    pub venue: HearingVenue,
    pub note: Option<HearingNote>,
    pub participants: Vec<HearingParticipantRef>,
    pub scheduling_basis: PrecautionaryHearingSchedulingBasis,
    pub review_targets: Vec<PrecautionaryMeasureRef>,
}

/// Immutable appointment values; source scope and current access require application checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionaryHearingValues {
    purpose: PrecautionaryHearingPurpose,
    scheduled_at: HearingTime,
    modality: HearingModality,
    venue: HearingVenue,
    note: Option<HearingNote>,
    participants: Vec<HearingParticipantRef>,
    scheduling_basis: PrecautionaryHearingSchedulingBasis,
    review_targets: Vec<PrecautionaryMeasureRef>,
}

impl PrecautionaryHearingValues {
    pub fn new(mut input: PrecautionaryHearingValuesInput) -> Result<Self, DomainError> {
        if input.participants.len() > MAX_PRECAUTIONARY_HEARING_PARTICIPANTS {
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
        let target_count = input.review_targets.len();
        let valid_targets = match input.purpose {
            PrecautionaryHearingPurpose::Imposition => target_count == 0,
            PrecautionaryHearingPurpose::Review => {
                (1..=MAX_PRECAUTIONARY_REVIEW_TARGETS).contains(&target_count)
            }
        };
        if !valid_targets {
            return Err(DomainError::InvalidHearingValue("review_targets"));
        }
        input
            .review_targets
            .sort_unstable_by_key(|reference| reference.id().as_uuid());
        if input
            .review_targets
            .windows(2)
            .any(|pair| pair[0].id() == pair[1].id())
        {
            return Err(DomainError::InvalidHearingValue("review_targets"));
        }
        Ok(Self {
            purpose: input.purpose,
            scheduled_at: input.scheduled_at,
            modality: input.modality,
            venue: input.venue,
            note: input.note,
            participants: input.participants,
            scheduling_basis: input.scheduling_basis,
            review_targets: input.review_targets,
        })
    }

    pub const fn purpose(&self) -> PrecautionaryHearingPurpose {
        self.purpose
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
    pub const fn scheduling_basis(&self) -> &PrecautionaryHearingSchedulingBasis {
        &self.scheduling_basis
    }
    pub fn review_targets(&self) -> &[PrecautionaryMeasureRef] {
        &self.review_targets
    }
}
