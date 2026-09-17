mod deadline_profile_definition_support;

use application::deadline_profiles::{
    DeadlineCompletionPolicy, DeadlineProfileDefinition, DeadlineProfileScope,
};
use deadline_profile_definition_support::{condition, example, id, input, scope, source};

#[test]
fn a_case_profile_preserves_its_declared_fields_and_exact_example() {
    let input = input();
    let title = input.title.clone();
    let description = input.description.clone();
    let scope = input.scope.clone();
    let references = input.references.clone();
    let trigger = input.trigger;
    let template = input.template;
    let completion = input.completion.clone();
    let conditions = input.conditions.clone();
    let examples = input.examples.clone();
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(profile.title(), &title);
    assert_eq!(profile.description(), &description);
    assert_eq!(profile.scope(), &scope);
    assert_eq!(profile.references(), references.as_slice());
    assert_eq!(profile.trigger(), trigger);
    assert_eq!(profile.template(), template);
    assert_eq!(profile.completion(), &completion);
    assert_eq!(profile.conditions(), conditions.as_slice());
    assert_eq!(profile.examples(), examples.as_slice());
}

#[test]
fn a_global_profile_retains_its_explicit_jurisdictional_scope() {
    let mut input = input();
    input.scope = DeadlineProfileScope::Global(scope());
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(profile.scope(), &DeadlineProfileScope::Global(scope()));
}

#[test]
fn nil_identifiers_are_values_not_missing_references() {
    let mut input = input();
    input.conditions[0].id = id(0);
    input.examples[0].id = id(0);
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(profile.references()[0].id(), id(0));
    assert_eq!(profile.conditions()[0].id, id(0));
    assert_eq!(profile.examples()[0].id, id(0));
}

#[test]
fn definition_preserves_collection_order_and_reference_order() {
    let mut input = input();
    input.references = vec![source(9), source(0), source(3)];
    input.conditions = vec![condition(9), condition(2)];
    input.examples = vec![example(8), example(1)];
    input.conditions[0].reference_ids = vec![id(3), id(9), id(0)];
    input.examples[0].reference_ids = vec![id(9), id(0), id(3)];
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(
        profile
            .references()
            .iter()
            .map(|item| item.id())
            .collect::<Vec<_>>(),
        vec![id(9), id(0), id(3)],
    );
    assert_eq!(
        profile
            .conditions()
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![id(9), id(2)],
    );
    assert_eq!(
        profile
            .examples()
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![id(8), id(1)],
    );
    assert_eq!(
        profile.conditions()[0].reference_ids,
        vec![id(3), id(9), id(0)]
    );
    assert_eq!(
        profile.examples()[0].reference_ids,
        vec![id(9), id(0), id(3)]
    );
}

#[test]
fn each_required_collection_accepts_sixteen_entries_and_rejects_zero_or_seventeen() {
    for count in [0, 16, 17] {
        for selected in 0..3 {
            let mut input = input();
            match selected {
                0 => input.references = (0..count).map(source).collect(),
                1 => input.conditions = (0..count).map(condition).collect(),
                2 => input.examples = (0..count).map(example).collect(),
                _ => unreachable!(),
            }
            assert_eq!(DeadlineProfileDefinition::new(input).is_ok(), count == 16);
        }
    }
}

#[test]
fn duplicate_identifiers_are_rejected_in_each_collection() {
    for selected in 0..3 {
        let mut input = input();
        match selected {
            0 => input.references.push(input.references[0].clone()),
            1 => input.conditions.push(input.conditions[0].clone()),
            2 => input.examples.push(input.examples[0].clone()),
            _ => unreachable!(),
        }
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}

#[test]
fn condition_references_must_be_nonempty_distinct_and_declared() {
    for references in [vec![], vec![id(0), id(0)], vec![id(99)]] {
        let mut input = input();
        input.conditions[0].reference_ids = references;
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}

#[test]
fn example_references_must_be_nonempty_distinct_and_declared() {
    for references in [vec![], vec![id(0), id(0)], vec![id(99)]] {
        let mut input = input();
        input.examples[0].reference_ids = references;
        assert!(DeadlineProfileDefinition::new(input).is_err());
    }
}

#[test]
fn civil_templates_reject_instant_completion() {
    let mut input = input();
    input.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    assert!(DeadlineProfileDefinition::new(input).is_err());
}
