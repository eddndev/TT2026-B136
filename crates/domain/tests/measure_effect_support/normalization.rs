use domain::crypto::Sha256Digest;
use domain::precautionary_hearings::{MeasureRevision, PrecautionaryMeasureRef};
use domain::precautionary_measures::{MeasureEffect, MeasureValues};

use crate::support::*;

#[test]
fn all_effects_preserve_their_normalized_material() {
    let expected = mixed();
    let outcome = changes(expected.clone());
    assert_eq!(outcome.changes().unwrap(), expected);
    assert_eq!(outcome.affected_ids(), (1..=9).map(id).collect::<Vec<_>>());
}

#[test]
fn effect_order_does_not_change_normalized_values_or_bytes() {
    let expected = changes(mixed());
    let mut effects = mixed();
    for _ in 0..effects.len() {
        effects.rotate_left(1);
        let reordered = changes(effects.clone());
        assert_eq!(reordered.changes(), expected.changes());
        assert_eq!(reordered.canonical_bytes(), expected.canonical_bytes());
    }
    effects.reverse();
    assert_eq!(
        changes(effects).canonical_bytes(),
        expected.canonical_bytes()
    );
}

#[test]
fn substitution_children_sort_and_its_smallest_identity_orders_the_effect() {
    let expected = vec![
        MeasureEffect::Substitute {
            predecessors: vec![previous(8), previous(9)],
            successors: vec![proposal(1), proposal(2)],
        },
        MeasureEffect::Confirm {
            previous: previous(4),
        },
    ];
    let reversed = vec![
        expected[1].clone(),
        MeasureEffect::Substitute {
            predecessors: vec![previous(9), previous(8)],
            successors: vec![proposal(2), proposal(1)],
        },
    ];
    let normalized = changes(reversed);
    assert_eq!(normalized.changes().unwrap(), expected);
    assert_eq!(
        normalized.affected_ids(),
        &[id(1), id(2), id(4), id(8), id(9)]
    );
    assert_eq!(
        normalized.canonical_bytes(),
        changes(expected).canonical_bytes()
    );
}

#[test]
fn technical_bound_counts_all_old_and_new_substitution_identities() {
    let accepted = MeasureEffect::Substitute {
        predecessors: (1..=16).map(previous).collect(),
        successors: (17..=32).map(proposal).collect(),
    };
    assert_eq!(changes(vec![accepted]).affected_ids().len(), 32);
    rejects(vec![MeasureEffect::Substitute {
        predecessors: (1..=16).map(previous).collect(),
        successors: (17..=33).map(proposal).collect(),
    }]);
}

#[test]
fn technical_bound_allows_32_effects_and_rejects_33() {
    let effects = (1..=32)
        .map(|value| MeasureEffect::Impose(proposal(value)))
        .collect();
    assert_eq!(changes(effects).affected_ids().len(), 32);
    rejects(
        (1..=33)
            .map(|value| MeasureEffect::Impose(proposal(value)))
            .collect(),
    );
}

#[test]
fn changes_and_each_substitution_side_must_be_nonempty() {
    rejects(vec![]);
    for (predecessors, successors) in [
        (vec![], vec![]),
        (vec![previous(1)], vec![]),
        (vec![], vec![proposal(2)]),
    ] {
        rejects(vec![MeasureEffect::Substitute {
            predecessors,
            successors,
        }]);
    }
}

#[test]
fn every_effect_rejects_an_identity_already_used_by_another_effect() {
    let collisions = vec![
        MeasureEffect::Impose(proposal(1)),
        MeasureEffect::Confirm {
            previous: previous(1),
        },
        MeasureEffect::Modify {
            previous: previous(1),
            values: MeasureValues::new(values_input()),
        },
        MeasureEffect::Revoke {
            previous: previous(1),
        },
        MeasureEffect::Cease {
            previous: previous(1),
        },
        MeasureEffect::Substitute {
            predecessors: vec![previous(1)],
            successors: vec![proposal(2)],
        },
        MeasureEffect::Substitute {
            predecessors: vec![previous(2)],
            successors: vec![proposal(1)],
        },
    ];
    for effect in collisions {
        rejects(vec![
            MeasureEffect::Confirm {
                previous: previous(1),
            },
            effect,
        ]);
    }
}

#[test]
fn different_exact_references_cannot_reuse_a_measure_identity() {
    let changed_revision = PrecautionaryMeasureRef::new(
        id(1),
        MeasureRevision::new(3).unwrap(),
        previous(1).digest(),
    );
    let changed_digest = PrecautionaryMeasureRef::new(
        id(1),
        previous(1).revision(),
        Sha256Digest::from_array([0x33; 32]),
    );
    for conflict in [previous(1), changed_revision, changed_digest] {
        rejects(vec![MeasureEffect::Substitute {
            predecessors: vec![previous(1), conflict],
            successors: vec![proposal(2)],
        }]);
        rejects(vec![
            MeasureEffect::Confirm {
                previous: previous(1),
            },
            MeasureEffect::Revoke { previous: conflict },
        ]);
    }
}

#[test]
fn duplicate_successor_ids_reject_even_when_the_values_differ() {
    let mut different = values_input();
    different.conditions = note("different conditions");
    let mut changed = proposal(2);
    changed.values = MeasureValues::new(different);
    for conflict in [proposal(2), changed] {
        rejects(vec![MeasureEffect::Substitute {
            predecessors: vec![previous(1)],
            successors: vec![proposal(2), conflict],
        }]);
    }
}

#[test]
fn substitutions_reject_old_new_overlap_within_and_across_effects() {
    rejects(vec![MeasureEffect::Substitute {
        predecessors: vec![previous(1)],
        successors: vec![proposal(1)],
    }]);
    rejects(vec![
        MeasureEffect::Substitute {
            predecessors: vec![previous(1)],
            successors: vec![proposal(2)],
        },
        MeasureEffect::Substitute {
            predecessors: vec![previous(2)],
            successors: vec![proposal(3)],
        },
    ]);
}
