use super::{
    decision_values::{Measure, Proposal},
    measure_request::{invalid, Reference},
    primitives::selected,
};
use crate::{error::ApiError, procedural_facts::object::Object};
use domain::{hearings::HearingNote, precautionary_measures::*};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Effect {
    Impose {
        proposal: Object<Proposal>,
    },
    Confirm {
        previous: Object<Reference>,
    },
    Modify {
        previous: Object<Reference>,
        values: Object<Measure>,
    },
    Revoke {
        previous: Object<Reference>,
    },
    Cease {
        previous: Object<Reference>,
    },
    Substitute {
        #[serde(deserialize_with = "selected")]
        predecessors: Vec<Object<Reference>>,
        #[serde(deserialize_with = "selected")]
        successors: Vec<Object<Proposal>>,
    },
}
impl Effect {
    fn validate(self) -> Result<MeasureEffect, ApiError> {
        Ok(match self {
            Self::Impose { proposal } => MeasureEffect::Impose(proposal.0.validate()?),
            Self::Confirm { previous } => MeasureEffect::Confirm {
                previous: previous.0.validate()?,
            },
            Self::Modify { previous, values } => MeasureEffect::Modify {
                previous: previous.0.validate()?,
                values: values.0.validate()?,
            },
            Self::Revoke { previous } => MeasureEffect::Revoke {
                previous: previous.0.validate()?,
            },
            Self::Cease { previous } => MeasureEffect::Cease {
                previous: previous.0.validate()?,
            },
            Self::Substitute {
                predecessors,
                successors,
            } => MeasureEffect::Substitute {
                predecessors: predecessors
                    .into_iter()
                    .map(|v| v.0.validate())
                    .collect::<Result<_, _>>()?,
                successors: successors
                    .into_iter()
                    .map(|v| v.0.validate())
                    .collect::<Result<_, _>>()?,
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Changes {
        #[serde(deserialize_with = "selected")]
        effects: Vec<Object<Effect>>,
    },
    NoMeasureChange {
        statement: String,
    },
}
impl Outcome {
    pub(super) fn validate(self) -> Result<MeasureDecisionOutcome, ApiError> {
        let input = match self {
            Self::Changes { effects } => MeasureDecisionOutcomeInput::Changes(
                effects
                    .into_iter()
                    .map(|v| v.0.validate())
                    .collect::<Result<_, _>>()?,
            ),
            Self::NoMeasureChange { statement } => MeasureDecisionOutcomeInput::NoMeasureChange(
                HearingNote::new(&statement).map_err(|_| invalid())?,
            ),
        };
        MeasureDecisionOutcome::new(input).map_err(|_| invalid())
    }
}
