use application::deadline_tracking::{
    DeadlineReviewState as State, TrackingDependency as Dependency, TrackingPolicies,
    TrackingPolicy as Policy, TrackingReview, TrackingReviewReason as Reason,
    TrackingReviewRequirement as Requirement,
};

const DEPENDENCIES: [Dependency; 3] = [
    Dependency::Profile,
    Dependency::Source,
    Dependency::Calendar,
];

fn requirement(dependency: Dependency, reason: Reason) -> Requirement {
    Requirement { dependency, reason }
}

fn policies(value: Policy) -> TrackingPolicies {
    TrackingPolicies {
        profile: value,
        source: value,
        calendar: value,
    }
}

fn set_policy(values: &mut TrackingPolicies, dependency: Dependency, policy: Policy) {
    match dependency {
        Dependency::Profile => values.profile = policy,
        Dependency::Source => values.source = policy,
        Dependency::Calendar => values.calendar = policy,
    }
}

fn pending(reasons: Vec<Requirement>) -> TrackingReview {
    TrackingReview::new(State::Pending, reasons).unwrap()
}

fn all_pairs() -> Vec<Requirement> {
    vec![
        requirement(Dependency::Profile, Reason::ProfileChanged),
        requirement(Dependency::Profile, Reason::DependencyRetired),
        requirement(Dependency::Profile, Reason::PolicyUndetermined),
        requirement(Dependency::Source, Reason::SourceChanged),
        requirement(Dependency::Source, Reason::DependencyRetired),
        requirement(Dependency::Source, Reason::PolicyUndetermined),
        requirement(Dependency::Calendar, Reason::DependencyRetired),
        requirement(Dependency::Calendar, Reason::PolicyUndetermined),
    ]
}

#[test]
fn accepted_and_legacy_require_no_reasons_while_pending_requires_a_reason() {
    for state in [State::Accepted, State::LegacyUndeclared] {
        let review = TrackingReview::new(state, vec![]).unwrap();
        assert_eq!(review.state(), state);
        assert!(review.reasons().is_empty());
        assert!(TrackingReview::new(
            state,
            vec![requirement(Dependency::Source, Reason::SourceChanged)],
        )
        .is_err());
    }
    assert!(TrackingReview::new(State::Pending, vec![]).is_err());
    let reasons = vec![requirement(Dependency::Source, Reason::SourceChanged)];
    let review = pending(reasons.clone());
    assert_eq!(review.state(), State::Pending);
    assert_eq!(review.reasons(), reasons.as_slice());
}

#[test]
fn constructor_accepts_eight_typed_canonical_pairs_without_inferring_policies() {
    let reasons = all_pairs();
    let review = pending(reasons.clone());
    assert_eq!(review.reasons(), reasons.as_slice());
    let mut too_many = reasons;
    too_many.push(requirement(
        Dependency::Calendar,
        Reason::PolicyUndetermined,
    ));
    assert!(TrackingReview::new(State::Pending, too_many).is_err());
}

#[test]
fn changed_reasons_cannot_be_assigned_to_another_dependency() {
    for (dependency, reason) in [
        (Dependency::Profile, Reason::SourceChanged),
        (Dependency::Calendar, Reason::SourceChanged),
        (Dependency::Source, Reason::ProfileChanged),
        (Dependency::Calendar, Reason::ProfileChanged),
    ] {
        assert!(
            TrackingReview::new(State::Pending, vec![requirement(dependency, reason)],).is_err()
        );
    }
}

#[test]
fn duplicate_reasons_are_rejected_instead_of_silently_deduplicated() {
    for value in all_pairs() {
        let duplicate = requirement(value.dependency, value.reason);
        assert!(TrackingReview::new(State::Pending, vec![value, duplicate]).is_err());
    }
}

#[test]
fn dependency_order_precedes_reason_order_and_is_not_normalized() {
    let canonical = vec![
        requirement(Dependency::Profile, Reason::PolicyUndetermined),
        requirement(Dependency::Source, Reason::SourceChanged),
    ];
    assert!(TrackingReview::new(State::Pending, canonical.clone()).is_ok());
    let mut reversed = canonical;
    reversed.reverse();
    assert!(TrackingReview::new(State::Pending, reversed).is_err());
    for dependency in DEPENDENCIES {
        let canonical = vec![
            requirement(dependency, Reason::DependencyRetired),
            requirement(dependency, Reason::PolicyUndetermined),
        ];
        assert!(TrackingReview::new(State::Pending, canonical.clone()).is_ok());
        let mut reversed = canonical;
        reversed.reverse();
        assert!(TrackingReview::new(State::Pending, reversed).is_err());
    }
}

#[test]
fn legacy_requires_all_three_policies_to_remain_undeclared_even_when_absent() {
    let review = TrackingReview::new(State::LegacyUndeclared, vec![]).unwrap();
    for present in [[true; 3], [false; 3], [true, false, true]] {
        assert!(review
            .validate_policies(&policies(Policy::Undetermined), present)
            .is_ok());
        for dependency in DEPENDENCIES {
            for policy in [Policy::Follow, Policy::Fixed] {
                let mut values = policies(Policy::Undetermined);
                set_policy(&mut values, dependency, policy);
                assert!(review.validate_policies(&values, present).is_err());
            }
        }
    }
}

#[test]
fn accepted_requires_an_explicit_policy_for_every_present_dependency() {
    let review = TrackingReview::new(State::Accepted, vec![]).unwrap();
    for policy in [Policy::Follow, Policy::Fixed] {
        assert!(review
            .validate_policies(&policies(policy), [true; 3])
            .is_ok());
    }
    for dependency in DEPENDENCIES {
        let mut values = policies(Policy::Follow);
        set_policy(&mut values, dependency, Policy::Undetermined);
        assert!(review.validate_policies(&values, [true; 3]).is_err());
    }
}

#[test]
fn absent_dependencies_require_undetermined_policy_without_creating_a_review() {
    let review = TrackingReview::new(State::Accepted, vec![]).unwrap();
    for (index, dependency) in DEPENDENCIES.into_iter().enumerate() {
        let mut present = [true; 3];
        present[index] = false;
        let mut values = policies(Policy::Follow);
        set_policy(&mut values, dependency, Policy::Undetermined);
        assert!(review.validate_policies(&values, present).is_ok());
        for policy in [Policy::Follow, Policy::Fixed] {
            set_policy(&mut values, dependency, policy);
            assert!(review.validate_policies(&values, present).is_err());
        }
    }
    assert!(review
        .validate_policies(&policies(Policy::Undetermined), [false; 3])
        .is_ok());
}

#[test]
fn an_absent_dependency_cannot_carry_any_review_reason() {
    for (index, dependency) in DEPENDENCIES.into_iter().enumerate() {
        let mut present = [true; 3];
        present[index] = false;
        let mut values = policies(Policy::Follow);
        set_policy(&mut values, dependency, Policy::Undetermined);
        for reason in all_pairs()
            .into_iter()
            .filter(|value| value.dependency == dependency)
        {
            let review = pending(vec![reason]);
            assert!(review.validate_policies(&values, present).is_err());
        }
    }
}

#[test]
fn pending_requires_an_undetermined_reason_for_each_present_undeclared_policy() {
    let reasons: Vec<_> = DEPENDENCIES
        .into_iter()
        .map(|dependency| requirement(dependency, Reason::PolicyUndetermined))
        .collect();
    let values = policies(Policy::Undetermined);
    assert!(pending(reasons.clone())
        .validate_policies(&values, [true; 3])
        .is_ok());
    for omitted in 0..3 {
        let mut incomplete = reasons.clone();
        incomplete.remove(omitted);
        assert!(pending(incomplete)
            .validate_policies(&values, [true; 3])
            .is_err());
    }
}

#[test]
fn retirement_reason_does_not_substitute_for_a_missing_policy_declaration() {
    for dependency in DEPENDENCIES {
        let mut values = policies(Policy::Follow);
        set_policy(&mut values, dependency, Policy::Undetermined);
        let review = pending(vec![requirement(dependency, Reason::DependencyRetired)]);
        assert!(review.validate_policies(&values, [true; 3]).is_err());
    }
}

#[test]
fn explicit_follow_or_fixed_policy_cannot_claim_to_be_undetermined() {
    for dependency in DEPENDENCIES {
        let review = pending(vec![requirement(dependency, Reason::PolicyUndetermined)]);
        for policy in [Policy::Follow, Policy::Fixed] {
            let mut values = policies(Policy::Follow);
            set_policy(&mut values, dependency, policy);
            assert!(review.validate_policies(&values, [true; 3]).is_err());
        }
    }
}

#[test]
fn changed_source_and_profile_reasons_require_follow_policy() {
    for (dependency, reason) in [
        (Dependency::Profile, Reason::ProfileChanged),
        (Dependency::Source, Reason::SourceChanged),
    ] {
        let review = pending(vec![requirement(dependency, reason)]);
        assert!(review
            .validate_policies(&policies(Policy::Follow), [true; 3])
            .is_ok());
        for policy in [Policy::Fixed, Policy::Undetermined] {
            let mut values = policies(Policy::Follow);
            set_policy(&mut values, dependency, policy);
            assert!(review.validate_policies(&values, [true; 3]).is_err());
        }
    }
}

#[test]
fn retirement_is_compatible_with_each_policy_and_preserves_other_requirements() {
    for dependency in DEPENDENCIES {
        for policy in [Policy::Follow, Policy::Fixed, Policy::Undetermined] {
            let mut values = policies(Policy::Follow);
            set_policy(&mut values, dependency, policy);
            let mut reasons = vec![requirement(dependency, Reason::DependencyRetired)];
            if policy == Policy::Undetermined {
                reasons.push(requirement(dependency, Reason::PolicyUndetermined));
            }
            assert!(pending(reasons)
                .validate_policies(&values, [true; 3])
                .is_ok());
        }
    }
}

#[test]
fn mixed_dependencies_preserve_canonical_reasons_and_declared_policy_meanings() {
    let values = TrackingPolicies {
        profile: Policy::Follow,
        source: Policy::Fixed,
        calendar: Policy::Undetermined,
    };
    let reasons = vec![
        requirement(Dependency::Profile, Reason::ProfileChanged),
        requirement(Dependency::Source, Reason::DependencyRetired),
        requirement(Dependency::Calendar, Reason::PolicyUndetermined),
    ];
    let review = pending(reasons.clone());
    assert!(review.validate_policies(&values, [true; 3]).is_ok());
    assert_eq!(review.reasons(), reasons.as_slice());
}
