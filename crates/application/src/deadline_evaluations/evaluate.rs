use super::*;
use crate::{
    deadline_inputs::{extract_checked_deadline_inputs, DeadlineInputMaterial},
    deadline_profiles::{
        DeadlineCompletionPolicy, DeadlineProfileDefinition, DeadlineProfileScope,
    },
};
use domain::{
    crypto::DocumentHasher,
    deadline_arithmetic::{evaluate_deadline_arithmetic, ArithmeticOutcome},
    deadline_triggers::TriggerOutcome,
    procedural_facts::FactDeclaration,
};
use std::collections::HashSet;
use time::{OffsetDateTime, UtcOffset};

/// Integrity failures reject the evaluation even if its declarations would block activation.
pub fn evaluate_profiled_deadline(
    hasher: &dyn DocumentHasher,
    profile: &DeadlineProfileDefinition,
    input: &DeadlineEvaluationInput,
    material: &DeadlineInputMaterial,
) -> Result<ProfiledDeadlineEvaluation, DeadlineEvaluationError> {
    let checked = extract_checked_deadline_inputs(
        hasher,
        profile.trigger(),
        &input.selection,
        input.calendar,
        material,
    )?;
    if matches!(profile.scope(), DeadlineProfileScope::Case(id) if *id != input.selection.case_id) {
        return Err(DeadlineEvaluationError::Invalid("profile.scope"));
    }
    let mut blocks = applicability(profile, &input.qualification)?;
    let rule = match profile.template().instantiate(input.ordered_quantity) {
        Ok(rule) => Some(rule),
        Err(block) => {
            blocks.push(DeadlineEvaluationBlock::Rule(block));
            None
        }
    };
    let trigger = checked.extraction().clone();
    let arithmetic = match trigger.outcome() {
        TriggerOutcome::Extracted { at } => {
            rule.map(|rule| evaluate_deadline_arithmetic(rule, *at, checked.calendar()))
        }
        TriggerOutcome::Blocked(block) => {
            blocks.push(DeadlineEvaluationBlock::Trigger(*block));
            None
        }
    };
    let candidate = arithmetic
        .as_ref()
        .and_then(|arithmetic| complete(profile.completion(), *arithmetic.outcome(), &mut blocks));
    let due_at = candidate.filter(|_| blocks.is_empty());
    Ok(ProfiledDeadlineEvaluation {
        trigger,
        rule,
        arithmetic,
        due_at,
        blocks,
    })
}

fn applicability(
    profile: &DeadlineProfileDefinition,
    qualification: &DeadlineApplicability,
) -> Result<Vec<DeadlineEvaluationBlock>, DeadlineEvaluationError> {
    let allowed: HashSet<_> = profile
        .conditions()
        .iter()
        .map(|condition| condition.id)
        .collect();
    let mut seen = HashSet::new();
    if qualification.conditions.len() > 16
        || qualification
            .conditions
            .iter()
            .any(|answer| !allowed.contains(&answer.id) || !seen.insert(answer.id))
    {
        return Err(DeadlineEvaluationError::Invalid("qualification.conditions"));
    }
    let mut blocks = Vec::new();
    match qualification.scope_applies {
        FactDeclaration::Unknown(_) => blocks.push(DeadlineEvaluationBlock::ScopeUnknown),
        FactDeclaration::Known(false) => blocks.push(DeadlineEvaluationBlock::ScopeRejected),
        FactDeclaration::Known(true) => {}
    }
    match qualification.unresolved_incident {
        FactDeclaration::Unknown(_) => blocks.push(DeadlineEvaluationBlock::IncidentUnknown),
        FactDeclaration::Known(true) => blocks.push(DeadlineEvaluationBlock::UnresolvedIncident),
        FactDeclaration::Known(false) => {}
    }
    for condition in profile.conditions() {
        match qualification
            .conditions
            .iter()
            .find(|answer| answer.id == condition.id)
            .map(|answer| &answer.applies)
        {
            None => blocks.push(DeadlineEvaluationBlock::ConditionMissing(condition.id)),
            Some(FactDeclaration::Unknown(_)) => {
                blocks.push(DeadlineEvaluationBlock::ConditionUnknown(condition.id))
            }
            Some(FactDeclaration::Known(false)) => {
                blocks.push(DeadlineEvaluationBlock::ConditionRejected(condition.id))
            }
            Some(FactDeclaration::Known(true)) => {}
        }
    }
    Ok(blocks)
}

fn complete(
    policy: &DeadlineCompletionPolicy,
    outcome: ArithmeticOutcome,
    blocks: &mut Vec<DeadlineEvaluationBlock>,
) -> Option<OffsetDateTime> {
    match outcome {
        ArithmeticOutcome::Blocked(block) => {
            blocks.push(DeadlineEvaluationBlock::Arithmetic(block));
            None
        }
        ArithmeticOutcome::InstantCandidate { instant } => Some(instant.to_offset(UtcOffset::UTC)),
        ArithmeticOutcome::CivilCandidate { date } => {
            let DeadlineCompletionPolicy::CivilCutoff(cutoff) = policy else {
                blocks.push(DeadlineEvaluationBlock::CivilCutoffMissing);
                return None;
            };
            if date < cutoff.from() || date > cutoff.through() {
                blocks.push(DeadlineEvaluationBlock::CutoffOutsideCoverage { candidate: date });
                return None;
            }
            // The cutoff constructor validates both UTC endpoints and its fixed offset.
            Some(
                date.date()
                    .with_time(cutoff.time())
                    .assume_offset(cutoff.offset())
                    .to_offset(UtcOffset::UTC),
            )
        }
    }
}
