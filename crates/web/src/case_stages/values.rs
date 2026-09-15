use super::{date::DeclaredTime, request::Support};
use crate::error::ApiError;
use application::case_stages::{CaseStageChange, StageTransition};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum Values {
    Adoption {
        stage: &'static str,
        known_at: DeclaredTime,
        reason: String,
        support: Support,
    },
    ToIntermediate {
        accusation_declared_at: DeclaredTime,
        accusation: Support,
        note: Option<String>,
    },
    ToTrial {
        opening_order_issued_at: DeclaredTime,
        opening_order: Support,
        received_at: DeclaredTime,
        receiving_court: String,
        receipt_reference: Option<String>,
        receipt_support: Option<Support>,
        note: Option<String>,
    },
}
impl TryFrom<CaseStageChange> for Values {
    type Error = ApiError;
    fn try_from(value: CaseStageChange) -> Result<Self, Self::Error> {
        Ok(match value {
            CaseStageChange::Adopt(value) => Self::Adoption {
                stage: value.stage().as_str(),
                known_at: value.known_at().try_into()?,
                reason: value.reason().as_str().into(),
                support: value.support().into(),
            },
            CaseStageChange::Transition(StageTransition::ToIntermediate(value)) => {
                Self::ToIntermediate {
                    accusation_declared_at: value.accusation_declared_at().try_into()?,
                    accusation: value.accusation().into(),
                    note: value.note().map(|v| v.as_str().into()),
                }
            }
            CaseStageChange::Transition(StageTransition::ToTrial(value)) => Self::ToTrial {
                opening_order_issued_at: value.opening_order_issued_at().try_into()?,
                opening_order: value.opening_order().into(),
                received_at: value.received_at().try_into()?,
                receiving_court: value.receiving_court().as_str().into(),
                receipt_reference: value.receipt_reference().map(|v| v.as_str().into()),
                receipt_support: value.receipt_support().map(Into::into),
                note: value.note().map(|v| v.as_str().into()),
            },
        })
    }
}
