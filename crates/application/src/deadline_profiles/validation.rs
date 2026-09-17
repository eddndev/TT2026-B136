use super::*;
use domain::{
    deadline_arithmetic::{evaluate_deadline_arithmetic, ArithmeticOutcome, ArithmeticRule},
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    typed_participants::Uuid,
};
use std::collections::HashSet;

pub(super) fn validate(
    values: &DeadlineProfileDefinitionInput,
) -> Result<(), DeadlineProfileError> {
    let references = unique(values.references.iter().map(|v| v.id()), "references")?;
    unique(values.conditions.iter().map(|v| v.id), "conditions")?;
    unique(values.examples.iter().map(|v| v.id), "examples")?;
    for condition in &values.conditions {
        linked(
            &condition.reference_ids,
            &references,
            "condition.references",
        )?;
    }
    let hours = matches!(
        values.template,
        DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours { .. })
            | DeadlineRuleTemplate::Ordered {
                unit: OrderedDeadlineUnit::ElapsedHours,
                ..
            }
    );
    if hours
        != matches!(
            values.completion,
            DeadlineCompletionPolicy::ArithmeticInstant
        )
    {
        return Err(DeadlineProfileError::Invalid("completion.unit"));
    }
    if let DeadlineCompletionPolicy::CivilCutoff(cutoff) = &values.completion {
        if !references.contains(&cutoff.reference_id()) {
            return Err(DeadlineProfileError::Invalid("completion.reference"));
        }
    }
    let mut successful = false;
    for example in &values.examples {
        linked(&example.reference_ids, &references, "example.references")?;
        let observed = match values.template.instantiate(example.ordered_quantity) {
            Err(block) => DeadlineExampleExpected::RuleBlocked(block),
            Ok(rule) => DeadlineExampleExpected::Arithmetic(
                *evaluate_deadline_arithmetic(rule, example.anchor, example.calendar.as_ref())
                    .outcome(),
            ),
        };
        if observed != example.expected {
            return Err(DeadlineProfileError::ExampleMismatch(example.id));
        }
        successful |= matches!(
            observed,
            DeadlineExampleExpected::Arithmetic(
                ArithmeticOutcome::CivilCandidate { .. }
                    | ArithmeticOutcome::InstantCandidate { .. }
            )
        );
    }
    if !successful {
        return Err(DeadlineProfileError::NoSuccessfulExample);
    }
    Ok(())
}
fn unique(
    ids: impl Iterator<Item = Uuid>,
    field: &'static str,
) -> Result<HashSet<Uuid>, DeadlineProfileError> {
    let mut values = HashSet::new();
    for id in ids {
        if values.len() == 16 || !values.insert(id) {
            return Err(DeadlineProfileError::Invalid(field));
        }
    }
    if values.is_empty() {
        return Err(DeadlineProfileError::Invalid(field));
    }
    Ok(values)
}
fn linked(
    ids: &[Uuid],
    references: &HashSet<Uuid>,
    field: &'static str,
) -> Result<(), DeadlineProfileError> {
    let selected = unique(ids.iter().copied(), field)?;
    if !selected.is_subset(references) {
        return Err(DeadlineProfileError::Invalid(field));
    }
    Ok(())
}
