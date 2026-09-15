use ::time::OffsetDateTime;

use crate::DomainError;

use super::{time::utc_in_range, CaseStage, StageAdoption, StageSupportRef, StageTransition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseStageChange {
    Adopt(StageAdoption),
    Transition(StageTransition),
}

impl CaseStageChange {
    pub const fn stage(&self) -> CaseStage {
        match self {
            Self::Adopt(values) => values.stage(),
            Self::Transition(values) => values.stage(),
        }
    }

    /// Returns distinct exact supports in first-role order for one validation batch.
    pub fn supports(&self) -> Vec<StageSupportRef> {
        match self {
            Self::Adopt(values) => vec![values.support()],
            Self::Transition(StageTransition::ToIntermediate(values)) => vec![values.accusation()],
            Self::Transition(StageTransition::ToTrial(values)) => {
                let mut supports = vec![values.opening_order()];
                if let Some(receipt) = values.receipt_support() {
                    if receipt.reference() != values.opening_order().reference() {
                        supports.push(receipt);
                    }
                }
                supports
            }
        }
    }

    /// Rejects only declarations whose earliest possible time is after capture.
    pub fn validate_recording_at(&self, recorded_at: OffsetDateTime) -> Result<(), DomainError> {
        let recorded_at = utc_in_range(recorded_at)?;
        let future = match self {
            Self::Adopt(values) => values.known_at().lower_bound() > recorded_at,
            Self::Transition(StageTransition::ToIntermediate(values)) => {
                values.accusation_declared_at().lower_bound() > recorded_at
            }
            Self::Transition(StageTransition::ToTrial(values)) => {
                values.opening_order_issued_at().lower_bound() > recorded_at
                    || values.received_at().lower_bound() > recorded_at
            }
        };
        if future {
            return Err(DomainError::StageActInFuture);
        }
        Ok(())
    }
}
