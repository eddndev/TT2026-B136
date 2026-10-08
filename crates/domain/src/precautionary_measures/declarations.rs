use super::{MeasureKind, MeasureTime, MeasureValidity};
use crate::{
    hearings::{HearingNote, HearingParticipantRef, HearingSupportRef},
    typed_participants::SubjectRevisionRef,
};

/// Declared supervisor selection; a directory title does not certify appointment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureSupervision {
    Known {
        participant: HearingParticipantRef,
        statement: HearingNote,
    },
    Unknown {
        reason: HearingNote,
    },
}

#[derive(Debug, Clone)]
pub struct MeasureValuesInput {
    pub subject: SubjectRevisionRef,
    pub kind: MeasureKind,
    pub conditions: HearingNote,
    pub validity: MeasureValidity,
    pub supervision: MeasureSupervision,
}

/// Complete declared terms; source existence and case scope require application checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureValues {
    subject: SubjectRevisionRef,
    kind: MeasureKind,
    conditions: HearingNote,
    validity: MeasureValidity,
    supervision: MeasureSupervision,
}
impl MeasureValues {
    pub fn new(input: MeasureValuesInput) -> Self {
        Self {
            subject: input.subject,
            kind: input.kind,
            conditions: input.conditions,
            validity: input.validity,
            supervision: input.supervision,
        }
    }
    pub const fn subject(&self) -> SubjectRevisionRef {
        self.subject
    }
    pub const fn kind(&self) -> MeasureKind {
        self.kind
    }
    pub const fn conditions(&self) -> &HearingNote {
        &self.conditions
    }
    pub const fn validity(&self) -> &MeasureValidity {
        &self.validity
    }
    pub const fn supervision(&self) -> &MeasureSupervision {
        &self.supervision
    }
}

#[derive(Debug, Clone)]
pub struct MeasureDecisionValuesInput {
    pub authority: HearingNote,
    pub declared_at: MeasureTime,
    pub justification: HearingNote,
    pub support: HearingSupportRef,
    pub locator: HearingNote,
}

/// Factual decision declarations, separate from effects, origin and capture clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionValues {
    authority: HearingNote,
    declared_at: MeasureTime,
    justification: HearingNote,
    support: HearingSupportRef,
    locator: HearingNote,
}
impl MeasureDecisionValues {
    pub fn new(input: MeasureDecisionValuesInput) -> Self {
        Self {
            authority: input.authority,
            declared_at: input.declared_at,
            justification: input.justification,
            support: input.support,
            locator: input.locator,
        }
    }
    pub const fn authority(&self) -> &HearingNote {
        &self.authority
    }
    pub const fn declared_at(&self) -> &MeasureTime {
        &self.declared_at
    }
    pub const fn justification(&self) -> &HearingNote {
        &self.justification
    }
    pub const fn support(&self) -> HearingSupportRef {
        self.support
    }
    pub const fn locator(&self) -> &HearingNote {
        &self.locator
    }
}
