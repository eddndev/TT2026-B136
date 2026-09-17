use crate::{
    cases::CaseId,
    procedural_facts::{
        FactDeclaration, FactHearingRef, FactLabel, FactResolutionRef, FactRevision, FactText,
        NotificationId,
    },
    procedural_time::DeclaredProceduralTime,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerFamily {
    Resolution,
    Notification,
    HearingResult,
}

/// Exact source selection, including the notification parent or selected agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerSourceRef {
    Resolution(FactResolutionRef),
    Notification {
        id: NotificationId,
        revision: FactRevision,
        resolution: FactResolutionRef,
    },
    HearingResult(FactHearingRef),
}
impl TriggerSourceRef {
    pub const fn family(self) -> TriggerFamily {
        match self {
            Self::Resolution(_) => TriggerFamily::Resolution,
            Self::Notification { .. } => TriggerFamily::Notification,
            Self::HearingResult(_) => TriggerFamily::HearingResult,
        }
    }
}

/// Each field has its own meaning; an absent field has no fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerField {
    ResolutionIssuedAt,
    NotificationPracticedAt,
    NotificationReceivedAt,
    NotificationStatedEffectAt,
    HearingSessionEventTime,
}
impl TriggerField {
    pub const fn family(self) -> TriggerFamily {
        match self {
            Self::ResolutionIssuedAt => TriggerFamily::Resolution,
            Self::NotificationPracticedAt
            | Self::NotificationReceivedAt
            | Self::NotificationStatedEffectAt => TriggerFamily::Notification,
            Self::HearingSessionEventTime => TriggerFamily::HearingResult,
        }
    }
}

/// A declared temporal purpose, without deciding whether it has legal effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualifiedTriggerPurpose {
    HearingEnd,
    OrderedPeriodStart,
}

/// Belongs to the same exact source as its enclosing selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedTriggerTime {
    pub purpose: QualifiedTriggerPurpose,
    pub at: DeclaredProceduralTime,
    pub statement: FactText,
    pub locator: FactLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerRequirement {
    SourceField(TriggerField),
    Qualified {
        purpose: QualifiedTriggerPurpose,
        family: TriggerFamily,
    },
}
impl TriggerRequirement {
    pub const fn family(self) -> TriggerFamily {
        match self {
            Self::SourceField(field) => field.family(),
            Self::Qualified { family, .. } => family,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerSelection {
    pub case_id: CaseId,
    pub source: FactDeclaration<TriggerSourceRef>,
    pub qualification: Option<QualifiedTriggerTime>,
}
