use crate::DomainError;

use super::{
    CaseStage, DeclaredStageTime, StageCourt, StageNote, StageReceiptReference, StageSupportRef,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntermediateStageTransition {
    accusation_declared_at: DeclaredStageTime,
    accusation: StageSupportRef,
    note: Option<StageNote>,
}

impl IntermediateStageTransition {
    pub const fn accusation_declared_at(&self) -> DeclaredStageTime {
        self.accusation_declared_at
    }
    pub const fn accusation(&self) -> StageSupportRef {
        self.accusation
    }
    pub const fn note(&self) -> Option<&StageNote> {
        self.note.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialStageTransition {
    opening_order_issued_at: DeclaredStageTime,
    opening_order: StageSupportRef,
    received_at: DeclaredStageTime,
    receiving_court: StageCourt,
    receipt_reference: Option<StageReceiptReference>,
    receipt_support: Option<StageSupportRef>,
    note: Option<StageNote>,
}

impl TrialStageTransition {
    pub const fn opening_order_issued_at(&self) -> DeclaredStageTime {
        self.opening_order_issued_at
    }
    pub const fn opening_order(&self) -> StageSupportRef {
        self.opening_order
    }
    pub const fn received_at(&self) -> DeclaredStageTime {
        self.received_at
    }
    pub const fn receiving_court(&self) -> &StageCourt {
        &self.receiving_court
    }
    pub const fn receipt_reference(&self) -> Option<&StageReceiptReference> {
        self.receipt_reference.as_ref()
    }
    pub const fn receipt_support(&self) -> Option<StageSupportRef> {
        self.receipt_support
    }
    pub const fn note(&self) -> Option<&StageNote> {
        self.note.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageTransition {
    ToIntermediate(IntermediateStageTransition),
    ToTrial(TrialStageTransition),
}

impl StageTransition {
    pub const fn to_intermediate(
        accusation_declared_at: DeclaredStageTime,
        accusation: StageSupportRef,
        note: Option<StageNote>,
    ) -> Self {
        Self::ToIntermediate(IntermediateStageTransition {
            accusation_declared_at,
            accusation,
            note,
        })
    }

    pub fn to_trial(
        opening_order_issued_at: DeclaredStageTime,
        opening_order: StageSupportRef,
        received_at: DeclaredStageTime,
        receiving_court: StageCourt,
        receipt_reference: Option<StageReceiptReference>,
        receipt_support: Option<StageSupportRef>,
        note: Option<StageNote>,
    ) -> Result<Self, DomainError> {
        if opening_order_issued_at.lower_bound() > received_at.upper_bound() {
            return Err(DomainError::InvalidStageActOrder);
        }
        if receipt_support.is_some_and(|support| {
            support.reference() == opening_order.reference()
                && support.digest() != opening_order.digest()
        }) {
            return Err(DomainError::ConflictingStageSupport);
        }
        Ok(Self::ToTrial(TrialStageTransition {
            opening_order_issued_at,
            opening_order,
            received_at,
            receiving_court,
            receipt_reference,
            receipt_support,
            note,
        }))
    }

    pub const fn stage(&self) -> CaseStage {
        match self {
            Self::ToIntermediate(_) => CaseStage::Intermediate,
            Self::ToTrial(_) => CaseStage::Trial,
        }
    }
}
