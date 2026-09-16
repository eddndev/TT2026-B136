use super::{
    DeclaredHearingResultTime, HearingResultAgreement, HearingResultAttendee, HearingResultExtent,
    HearingResultOccurrence, HearingResultProvenance, HearingResultText,
};
use crate::DomainError;
use std::collections::HashSet;

pub const MAX_HEARING_RESULT_ATTENDEES: usize = 32;
pub const MAX_HEARING_RESULT_AGREEMENTS: usize = 16;

/// Construction input; only HearingResultValues guarantees cross-field invariants.
#[derive(Debug, Clone)]
pub struct HearingResultValuesInput {
    pub occurrence: HearingResultOccurrence,
    pub extent: HearingResultExtent,
    pub event_time: DeclaredHearingResultTime,
    pub summary: HearingResultText,
    pub attendees: Vec<HearingResultAttendee>,
    pub agreements: Vec<HearingResultAgreement>,
    pub provenance: HearingResultProvenance,
}

/// Normalized declarations; root bindings and captured administration are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingResultValues {
    occurrence: HearingResultOccurrence,
    extent: HearingResultExtent,
    event_time: DeclaredHearingResultTime,
    summary: HearingResultText,
    attendees: Vec<HearingResultAttendee>,
    agreements: Vec<HearingResultAgreement>,
    provenance: HearingResultProvenance,
}
impl HearingResultValues {
    pub fn new(mut input: HearingResultValuesInput) -> Result<Self, DomainError> {
        if input.occurrence == HearingResultOccurrence::NotStarted
            && input.extent != HearingResultExtent::Unspecified
        {
            return Err(DomainError::InvalidHearingResultValue("extent"));
        }
        if input.attendees.len() > MAX_HEARING_RESULT_ATTENDEES {
            return Err(DomainError::InvalidHearingResultValue("attendees"));
        }
        input
            .attendees
            .sort_unstable_by_key(|value| value.participant_id().as_uuid());
        if input
            .attendees
            .windows(2)
            .any(|pair| pair[0].participant_id() == pair[1].participant_id())
        {
            return Err(DomainError::InvalidHearingResultValue("attendees"));
        }
        let mut agreement_ids = HashSet::new();
        if input.agreements.len() > MAX_HEARING_RESULT_AGREEMENTS
            || input
                .agreements
                .iter()
                .any(|value| !agreement_ids.insert(value.id()))
        {
            return Err(DomainError::InvalidHearingResultValue("agreements"));
        }
        Ok(Self {
            occurrence: input.occurrence,
            extent: input.extent,
            event_time: input.event_time,
            summary: input.summary,
            attendees: input.attendees,
            agreements: input.agreements,
            provenance: input.provenance,
        })
    }
    pub const fn occurrence(&self) -> HearingResultOccurrence {
        self.occurrence
    }
    pub const fn extent(&self) -> HearingResultExtent {
        self.extent
    }
    pub const fn event_time(&self) -> DeclaredHearingResultTime {
        self.event_time
    }
    pub const fn summary(&self) -> &HearingResultText {
        &self.summary
    }
    pub fn attendees(&self) -> &[HearingResultAttendee] {
        &self.attendees
    }
    pub fn agreements(&self) -> &[HearingResultAgreement] {
        &self.agreements
    }
    pub const fn provenance(&self) -> &HearingResultProvenance {
        &self.provenance
    }
}
