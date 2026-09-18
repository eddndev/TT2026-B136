#[path = "validation_trace.rs"]
mod trace;

use super::{invalid, DeadlineEvaluationRecord};
use crate::{deadline_evaluations::DeadlineEvaluationBlock as Block, ApplicationError};
use domain::{deadline_arithmetic::ArithmeticOutcome, deadline_triggers::TriggerOutcome};

pub(super) fn validate(value: &DeadlineEvaluationRecord) -> Result<(), ApplicationError> {
    let rules = value
        .blocks
        .iter()
        .filter(|block| matches!(block, Block::Rule(_)))
        .count();
    if rules != usize::from(value.rule.is_none()) {
        return Err(invalid("rule presence and block disagree"));
    }
    let triggers: Vec<_> = value
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Trigger(cause) => Some(*cause),
            _ => None,
        })
        .collect();
    match value.trigger_outcome {
        TriggerOutcome::Blocked(cause) => {
            if triggers != [cause] || value.arithmetic.is_some() {
                return Err(invalid("blocked trigger has arithmetic or wrong cause"));
            }
        }
        TriggerOutcome::Extracted { .. } if !triggers.is_empty() => {
            return Err(invalid("extracted trigger has a block"))
        }
        _ => {}
    }
    let arithmetic_blocks: Vec<_> = value
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Arithmetic(cause) => Some(*cause),
            _ => None,
        })
        .collect();
    if let Some(arithmetic) = &value.arithmetic {
        if value.rule != Some(arithmetic.rule)
            || value.trigger_outcome
                != (TriggerOutcome::Extracted {
                    at: arithmetic.anchor,
                })
        {
            return Err(invalid(
                "arithmetic operands differ from captured trigger and rule",
            ));
        }
        trace::validate(arithmetic)?;
        match arithmetic.outcome {
            ArithmeticOutcome::Blocked(cause) if arithmetic_blocks != [cause] => {
                return Err(invalid("arithmetic block differs"))
            }
            ArithmeticOutcome::Blocked(_) => {}
            _ if !arithmetic_blocks.is_empty() => {
                return Err(invalid("successful arithmetic has arithmetic block"))
            }
            _ => {}
        }
    } else if !arithmetic_blocks.is_empty()
        || (value.rule.is_some()
            && matches!(value.trigger_outcome, TriggerOutcome::Extracted { .. }))
    {
        return Err(invalid("arithmetic result is missing"));
    }
    for block in &value.blocks {
        let outcome = value.arithmetic.as_ref().map(|value| value.outcome);
        match block {
            Block::CivilCutoffMissing
                if !matches!(outcome, Some(ArithmeticOutcome::CivilCandidate { .. })) =>
            {
                return Err(invalid("civil completion block has no civil candidate"));
            }
            Block::CutoffOutsideCoverage { candidate } if !matches!(outcome, Some(ArithmeticOutcome::CivilCandidate { date }) if date == *candidate) =>
            {
                return Err(invalid(
                    "cutoff block candidate differs from arithmetic result",
                ));
            }
            _ => {}
        }
    }
    match value.due_at {
        Some(due) => {
            if !value.blocks.is_empty() {
                return Err(invalid("due instant has blocking causes"));
            }
            match value.arithmetic.as_ref().map(|value| value.outcome) {
                Some(ArithmeticOutcome::InstantCandidate { instant }) if instant == due => {}
                Some(ArithmeticOutcome::CivilCandidate { .. }) => {}
                _ => return Err(invalid("due instant has no successful arithmetic result")),
            }
        }
        None if value.blocks.is_empty() => {
            return Err(invalid("missing due instant has no blocking cause"))
        }
        None => {}
    }
    Ok(())
}
