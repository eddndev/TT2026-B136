use super::MeasureValues;
use crate::{
    hearings::HearingNote,
    precautionary_hearings::{MeasureId, PrecautionaryMeasureRef},
    DomainError,
};
use std::collections::BTreeMap;

const MAX_AFFECTED_MEASURES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureProposal {
    pub id: MeasureId,
    pub values: MeasureValues,
}

/// Declared judicial effects; administrative corrections are separate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureEffect {
    Impose(MeasureProposal),
    Confirm {
        previous: PrecautionaryMeasureRef,
    },
    Modify {
        previous: PrecautionaryMeasureRef,
        values: MeasureValues,
    },
    Revoke {
        previous: PrecautionaryMeasureRef,
    },
    Cease {
        previous: PrecautionaryMeasureRef,
    },
    Substitute {
        predecessors: Vec<PrecautionaryMeasureRef>,
        successors: Vec<MeasureProposal>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureDecisionOutcomeInput {
    Changes(Vec<MeasureEffect>),
    NoMeasureChange(HearingNote),
}

/// Bounded, ordered intent. Predecessor existence and legal effects are not inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureDecisionOutcome {
    input: MeasureDecisionOutcomeInput,
    affected_ids: Vec<MeasureId>,
}

impl MeasureDecisionOutcome {
    pub fn new(mut input: MeasureDecisionOutcomeInput) -> Result<Self, DomainError> {
        let mut identities = BTreeMap::new();
        if let MeasureDecisionOutcomeInput::Changes(effects) = &mut input {
            if effects.is_empty() || effects.len() > MAX_AFFECTED_MEASURES {
                return Err(invalid("effect_count"));
            }
            for effect in effects.iter_mut() {
                if let MeasureEffect::Substitute {
                    predecessors,
                    successors,
                } = effect
                {
                    if predecessors.is_empty()
                        || successors.is_empty()
                        || predecessors.len() > MAX_AFFECTED_MEASURES
                        || successors.len() > MAX_AFFECTED_MEASURES
                    {
                        return Err(invalid("substitution_count"));
                    }
                    predecessors.sort_unstable_by_key(|item| item.id().as_uuid());
                    successors.sort_unstable_by_key(|item| item.id.as_uuid());
                }
                for id in effect_ids(effect) {
                    if identities.insert(id.as_uuid(), id).is_some() {
                        return Err(invalid("duplicate_measure_identity"));
                    }
                    if identities.len() > MAX_AFFECTED_MEASURES {
                        return Err(invalid("affected_measure_count"));
                    }
                }
            }
            effects.sort_unstable_by_key(|effect| {
                effect_ids(effect).into_iter().map(|id| id.as_uuid()).min()
            });
        }
        Ok(Self {
            input,
            affected_ids: identities.into_values().collect(),
        })
    }

    pub fn changes(&self) -> Option<&[MeasureEffect]> {
        match &self.input {
            MeasureDecisionOutcomeInput::Changes(effects) => Some(effects),
            MeasureDecisionOutcomeInput::NoMeasureChange(_) => None,
        }
    }

    pub fn no_measure_change(&self) -> Option<&HearingNote> {
        match &self.input {
            MeasureDecisionOutcomeInput::NoMeasureChange(statement) => Some(statement),
            MeasureDecisionOutcomeInput::Changes(_) => None,
        }
    }

    pub fn affected_ids(&self) -> &[MeasureId] {
        &self.affected_ids
    }
}

fn effect_ids(effect: &MeasureEffect) -> Vec<MeasureId> {
    match effect {
        MeasureEffect::Impose(proposal) => vec![proposal.id],
        MeasureEffect::Confirm { previous }
        | MeasureEffect::Modify { previous, .. }
        | MeasureEffect::Revoke { previous }
        | MeasureEffect::Cease { previous } => vec![previous.id()],
        MeasureEffect::Substitute {
            predecessors,
            successors,
        } => predecessors
            .iter()
            .map(|item| item.id())
            .chain(successors.iter().map(|item| item.id))
            .collect(),
    }
}

fn invalid(field: &'static str) -> DomainError {
    DomainError::InvalidPrecautionaryMeasure(field)
}
