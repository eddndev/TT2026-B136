use super::{
    QualifiedTriggerPurpose, TriggerFamily, TriggerField, TriggerRequirement, TriggerSelection,
    TriggerSourceSnapshot,
};
use crate::{
    deadline_arithmetic::{ArithmeticRule, DeadlineArithmetic},
    procedural_time::DeclaredProceduralTime,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TriggerIntegrityError {
    #[error("the selected source material is missing")]
    MissingMaterial,
    #[error("unknown source selection received material")]
    UnexpectedMaterial,
    #[error("the source material belongs to a different case")]
    CaseMismatch,
    #[error("source family, identity or revision differs from the selection")]
    SourceMismatch,
    #[error("the notification parent differs from its root, values or selection")]
    ParentMismatch,
    #[error("the selected agreement is absent from the exact result revision")]
    MissingAgreement,
    #[error("the source time cannot retain its declared components")]
    InvalidTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerBlock {
    UnknownSource,
    AbsentField(TriggerField),
    IncompatibleFamily {
        expected: TriggerFamily,
        actual: TriggerFamily,
    },
    MissingQualification {
        purpose: QualifiedTriggerPurpose,
    },
    QualificationMismatch {
        expected: QualifiedTriggerPurpose,
        actual: QualifiedTriggerPurpose,
    },
    UnexpectedQualification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerOutcome {
    Extracted { at: DeclaredProceduralTime },
    Blocked(TriggerBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerExtraction {
    pub(super) requirement: TriggerRequirement,
    pub(super) selection: TriggerSelection,
    pub(super) source: Option<TriggerSourceSnapshot>,
    pub(super) outcome: TriggerOutcome,
}
impl TriggerExtraction {
    pub const fn requirement(&self) -> TriggerRequirement {
        self.requirement
    }
    pub const fn selection(&self) -> &TriggerSelection {
        &self.selection
    }
    pub const fn source(&self) -> Option<&TriggerSourceSnapshot> {
        self.source.as_ref()
    }
    pub const fn outcome(&self) -> &TriggerOutcome {
        &self.outcome
    }
}

/// An extraction and its optional arithmetic result; this is not a persisted deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggeredArithmetic {
    pub(super) rule: ArithmeticRule,
    pub(super) trigger: TriggerExtraction,
    pub(super) arithmetic: Option<DeadlineArithmetic>,
}
impl TriggeredArithmetic {
    pub const fn rule(&self) -> ArithmeticRule {
        self.rule
    }
    pub const fn trigger(&self) -> &TriggerExtraction {
        &self.trigger
    }
    pub const fn arithmetic(&self) -> Option<&DeadlineArithmetic> {
        self.arithmetic.as_ref()
    }
}
