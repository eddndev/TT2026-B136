use domain::crypto::Sha256Digest;
use domain::hearings::HearingParticipantRef;
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_hearings::{MeasureRevision, PrecautionaryMeasureRef};
use domain::precautionary_measures::{
    MeasureDecisionOutcome, MeasureDecisionOutcomeInput, MeasureEffect, MeasureKind,
    MeasureSupervision, MeasureTime, MeasureValidity, MeasureValues, MeasureValuesInput,
};
use domain::procedural_time::DeclaredProceduralTime;
use domain::typed_participants::{CaseSubjectId, SubjectRevision};
use uuid::Uuid;

use crate::support::*;

fn wraps_values(values: MeasureValues) -> Vec<MeasureEffect> {
    let mut proposed = proposal(1);
    proposed.values = values.clone();
    vec![
        MeasureEffect::Impose(proposed.clone()),
        MeasureEffect::Modify {
            previous: previous(1),
            values,
        },
        MeasureEffect::Substitute {
            predecessors: vec![previous(2)],
            successors: vec![proposed],
        },
    ]
}

fn assert_values_bound(variants: Vec<MeasureValuesInput>, baseline: MeasureValuesInput) {
    let expected = wraps_values(MeasureValues::new(baseline));
    for input in variants {
        let changed = wraps_values(MeasureValues::new(input));
        for (old, new) in expected.iter().zip(changed) {
            assert_ne!(
                changes(vec![old.clone()]).canonical_bytes(),
                changes(vec![new]).canonical_bytes(),
            );
        }
    }
}

#[test]
fn proposal_and_modification_bind_each_complete_measure_value_field() {
    let mut variants = Vec::new();
    for mutate in [
        (|v: &mut MeasureValuesInput| {
            v.subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(101));
        }) as fn(&mut MeasureValuesInput),
        |v| v.subject.revision = SubjectRevision::new(2).unwrap(),
        |v| v.subject.values_digest = Sha256Digest::from_array([0x12; 32]),
        |v| v.kind = MeasureKind::PretrialDetention,
        |v| v.conditions = note("changed conditions"),
        |v| {
            v.supervision = MeasureSupervision::Unknown {
                reason: note("changed reason"),
            }
        },
        |v| {
            v.validity = MeasureValidity::new(
                MeasureTime::new(
                    DeclaredProceduralTime::unknown(),
                    Some(note("changed start")),
                )
                .unwrap(),
                note("V"),
                None,
            )
            .unwrap();
        },
        |v| {
            v.validity =
                MeasureValidity::new(v.validity.start().clone(), note("changed validity"), None)
                    .unwrap();
        },
        |v| {
            v.validity = MeasureValidity::new(
                v.validity.start().clone(),
                note("V"),
                Some(
                    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note("end"))).unwrap(),
                ),
            )
            .unwrap();
        },
    ] {
        let mut input = values_input();
        mutate(&mut input);
        variants.push(input);
    }
    assert_values_bound(variants, values_input());
}

fn supervisor(id: u128, revision: u32, statement: &str) -> MeasureSupervision {
    MeasureSupervision::Known {
        participant: HearingParticipantRef::new(
            ParticipantId::from_uuid(Uuid::from_u128(id)),
            ParticipantRevision::new(revision).unwrap(),
        ),
        statement: note(statement),
    }
}

#[test]
fn full_values_preserve_supervisor_identity_revision_statement_and_variant() {
    let mut baseline = values_input();
    baseline.supervision = supervisor(200, 1, "S");
    let variants = [
        supervisor(201, 1, "S"),
        supervisor(200, 2, "S"),
        supervisor(200, 1, "changed statement"),
        MeasureSupervision::Unknown { reason: note("S") },
    ]
    .into_iter()
    .map(|supervision| {
        let mut input = baseline.clone();
        input.supervision = supervision;
        input
    })
    .collect();
    assert_values_bound(variants, baseline);
}

fn wraps_previous(value: PrecautionaryMeasureRef) -> Vec<MeasureEffect> {
    vec![
        MeasureEffect::Confirm { previous: value },
        MeasureEffect::Modify {
            previous: value,
            values: MeasureValues::new(values_input()),
        },
        MeasureEffect::Revoke { previous: value },
        MeasureEffect::Cease { previous: value },
        MeasureEffect::Substitute {
            predecessors: vec![value],
            successors: vec![proposal(3)],
        },
    ]
}

#[test]
fn each_previous_reference_binds_identity_revision_and_capture_digest() {
    let original = previous(1);
    let variants = [
        previous(2),
        PrecautionaryMeasureRef::new(id(1), MeasureRevision::new(3).unwrap(), original.digest()),
        PrecautionaryMeasureRef::new(
            id(1),
            original.revision(),
            Sha256Digest::from_array([0x23; 32]),
        ),
    ];
    for variant in variants {
        for (old, new) in wraps_previous(original)
            .into_iter()
            .zip(wraps_previous(variant))
        {
            assert_ne!(
                changes(vec![old]).canonical_bytes(),
                changes(vec![new]).canonical_bytes()
            );
        }
    }
}

#[test]
fn proposal_identities_are_bound_in_imposition_and_substitution() {
    assert_ne!(
        changes(vec![MeasureEffect::Impose(proposal(1))]).canonical_bytes(),
        changes(vec![MeasureEffect::Impose(proposal(2))]).canonical_bytes(),
    );
    let substitution = |successor| MeasureEffect::Substitute {
        predecessors: vec![previous(3)],
        successors: vec![proposal(successor)],
    };
    assert_ne!(
        changes(vec![substitution(1)]).canonical_bytes(),
        changes(vec![substitution(2)]).canonical_bytes(),
    );
}

#[test]
fn confirm_revoke_and_cease_have_distinct_commitments() {
    let effects = [
        MeasureEffect::Confirm {
            previous: previous(1),
        },
        MeasureEffect::Revoke {
            previous: previous(1),
        },
        MeasureEffect::Cease {
            previous: previous(1),
        },
    ];
    for (index, effect) in effects.iter().enumerate() {
        for other in &effects[index + 1..] {
            assert_ne!(
                changes(vec![effect.clone()]).canonical_bytes(),
                changes(vec![other.clone()]).canonical_bytes(),
            );
        }
    }
}

#[test]
fn outcome_text_and_group_membership_are_bound() {
    let no_change = |statement| {
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(note(
            statement,
        )))
        .unwrap()
        .canonical_bytes()
    };
    assert_ne!(no_change("C"), no_change("C\u{e9}"));
    assert_ne!(no_change("C"), changes(mixed()).canonical_bytes());
    let complete = changes(mixed()).canonical_bytes();
    for index in 0..mixed().len() {
        let mut effects = mixed();
        effects.remove(index);
        assert_ne!(complete, changes(effects).canonical_bytes());
    }
}

#[test]
fn substitution_commits_every_predecessor_and_successor() {
    let complete = changes(vec![MeasureEffect::Substitute {
        predecessors: vec![previous(1), previous(2)],
        successors: vec![proposal(3), proposal(4)],
    }])
    .canonical_bytes();
    for (predecessors, successors) in [
        (vec![previous(1)], vec![proposal(3), proposal(4)]),
        (vec![previous(1), previous(2)], vec![proposal(3)]),
        (
            vec![previous(1), previous(5)],
            vec![proposal(3), proposal(4)],
        ),
        (
            vec![previous(1), previous(2)],
            vec![proposal(3), proposal(5)],
        ),
    ] {
        assert_ne!(
            complete,
            changes(vec![MeasureEffect::Substitute {
                predecessors,
                successors,
            }])
            .canonical_bytes()
        );
    }
}
