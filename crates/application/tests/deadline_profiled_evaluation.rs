#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_evaluation_support;
use application::{deadline_evaluations::*, deadline_profiles::*};
use deadline_evaluation_support::*;
use domain::{
    cases::CaseId, deadline_arithmetic::*, deadline_profiles::*, deadline_triggers::*,
    procedural_facts::*,
};
use time::UtcOffset;

#[test]
fn civil_cutoff_uses_its_own_offset_and_keeps_arithmetic_trace() {
    let (p, i, m) = fixture();
    let e = evaluate(p, &i, &m);
    assert!(e.blocks().is_empty());
    assert_eq!(
        e.due_at().unwrap(),
        date("2026-01-07")
            .date()
            .with_hms(23, 30, 0)
            .unwrap()
            .assume_utc()
    );
    assert_eq!(e.due_at().unwrap().offset(), UtcOffset::UTC);
    assert!(!e.arithmetic().unwrap().trace().is_empty());
    assert_eq!(e.trigger().selection(), &i.selection);
}
#[test]
fn candidate_only_retains_date_without_fabricating_midnight() {
    let (mut p, i, m) = fixture();
    p.completion = DeadlineCompletionPolicy::CivilCandidateOnly;
    let e = evaluate(p, &i, &m);
    assert_eq!(e.blocks(), &[DeadlineEvaluationBlock::CivilCutoffMissing]);
    assert!(e.due_at().is_none());
    assert_eq!(
        e.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-07")
        }
    );
}
#[test]
fn cutoff_coverage_is_inclusive_and_blocks_outside_candidate() {
    for (from, through, ok) in [
        ("2026-01-07", "2026-01-07", true),
        ("2026-01-08", "2026-02-01", false),
        ("2026-01-01", "2026-01-06", false),
    ] {
        let (mut p, i, m) = fixture();
        p.completion = cutoff(from, through);
        let e = evaluate(p, &i, &m);
        assert_eq!(e.due_at().is_some(), ok);
        if !ok {
            assert_eq!(
                e.blocks(),
                &[DeadlineEvaluationBlock::CutoffOutsideCoverage {
                    candidate: date("2026-01-07")
                }]
            );
        }
    }
}
#[test]
fn missing_ordered_quantity_never_uses_the_maximum() {
    let (mut p, i, m) = fixture();
    ordered(&mut p);
    let e = evaluate(p, &i, &m);
    assert_eq!(
        e.blocks(),
        &[DeadlineEvaluationBlock::Rule(
            DeadlineRuleBlock::MissingOrderedQuantity
        )]
    );
    assert!(e.rule().is_none());
    assert!(e.arithmetic().is_none());
    assert!(e.due_at().is_none());
    assert!(matches!(
        e.trigger().outcome(),
        TriggerOutcome::Extracted { .. }
    ));
}
#[test]
fn ordered_quantity_is_exact_and_overflow_is_not_clamped() {
    let (mut p, mut i, m) = fixture();
    ordered(&mut p);
    i.ordered_quantity = Some(n(3));
    let e = evaluate(p.clone(), &i, &m);
    assert!(e.blocks().is_empty());
    assert_eq!(
        e.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: date("2026-01-08")
        }
    );
    i.ordered_quantity = Some(n(7));
    let e = evaluate(p, &i, &m);
    assert_eq!(
        e.blocks(),
        &[DeadlineEvaluationBlock::Rule(
            DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
                maximum: n(6),
                supplied: n(7)
            }
        )]
    );
    assert!(e.due_at().is_none());
}
#[test]
fn fixed_profile_rejects_an_unexpected_ordered_quantity() {
    let (p, mut i, m) = fixture();
    i.ordered_quantity = Some(n(2));
    let e = evaluate(p, &i, &m);
    assert_eq!(
        e.blocks(),
        &[DeadlineEvaluationBlock::Rule(
            DeadlineRuleBlock::UnexpectedOrderedQuantity
        )]
    );
}
#[test]
fn all_applicability_blocks_preserve_the_mathematical_candidate() {
    let (p, mut i, m) = fixture();
    for (scope, incident, condition, expected) in [
        (
            FactDeclaration::Known(false),
            FactDeclaration::Known(false),
            FactDeclaration::Known(true),
            DeadlineEvaluationBlock::ScopeRejected,
        ),
        (
            FactDeclaration::Unknown(text("Unknown scope")),
            FactDeclaration::Known(false),
            FactDeclaration::Known(true),
            DeadlineEvaluationBlock::ScopeUnknown,
        ),
        (
            FactDeclaration::Known(true),
            FactDeclaration::Known(true),
            FactDeclaration::Known(true),
            DeadlineEvaluationBlock::UnresolvedIncident,
        ),
        (
            FactDeclaration::Known(true),
            FactDeclaration::Unknown(text("Unknown incident")),
            FactDeclaration::Known(true),
            DeadlineEvaluationBlock::IncidentUnknown,
        ),
        (
            FactDeclaration::Known(true),
            FactDeclaration::Known(false),
            FactDeclaration::Known(false),
            DeadlineEvaluationBlock::ConditionRejected(id(1)),
        ),
        (
            FactDeclaration::Known(true),
            FactDeclaration::Known(false),
            FactDeclaration::Unknown(text("Unknown condition")),
            DeadlineEvaluationBlock::ConditionUnknown(id(1)),
        ),
    ] {
        i.qualification.scope_applies = scope;
        i.qualification.unresolved_incident = incident;
        i.qualification.conditions[0].applies = condition;
        let e = evaluate(p.clone(), &i, &m);
        assert_eq!(e.blocks(), &[expected]);
        assert!(e.due_at().is_none());
        assert!(matches!(
            e.arithmetic().unwrap().outcome(),
            ArithmeticOutcome::CivilCandidate { .. }
        ));
    }
}
#[test]
fn missing_condition_is_a_block_but_unknown_and_duplicate_ids_are_invalid() {
    let (p, mut i, m) = fixture();
    let answer = i.qualification.conditions.remove(0);
    assert_eq!(
        evaluate(p.clone(), &i, &m).blocks(),
        &[DeadlineEvaluationBlock::ConditionMissing(id(1))]
    );
    for answers in [
        vec![DeadlineConditionAnswer {
            id: id(99),
            ..answer.clone()
        }],
        vec![answer.clone(), answer],
    ] {
        i.qualification.conditions = answers;
        assert!(matches!(
            evaluate_profiled_deadline(
                inputs::hasher().as_ref(),
                &DeadlineProfileDefinition::new(p.clone()).unwrap(),
                &i,
                &m
            ),
            Err(DeadlineEvaluationError::Invalid(_))
        ));
    }
}
#[test]
fn profile_for_another_case_is_rejected() {
    let (mut p, i, m) = fixture();
    p.scope = DeadlineProfileScope::Case(CaseId::from_uuid(id(999)));
    assert!(matches!(
        evaluate_profiled_deadline(
            inputs::hasher().as_ref(),
            &DeadlineProfileDefinition::new(p).unwrap(),
            &i,
            &m
        ),
        Err(DeadlineEvaluationError::Invalid(_))
    ));
}
#[test]
fn corruption_cannot_hide_behind_missing_quantity_or_rejected_applicability() {
    let (mut p, mut i, mut m) = fixture();
    ordered(&mut p);
    i.qualification.scope_applies = FactDeclaration::Known(false);
    m.case_id = CaseId::from_uuid(id(999));
    assert!(matches!(
        evaluate_profiled_deadline(
            inputs::hasher().as_ref(),
            &DeadlineProfileDefinition::new(p).unwrap(),
            &i,
            &m
        ),
        Err(DeadlineEvaluationError::Inputs(_))
    ));
}
#[test]
fn absent_source_preserves_independent_quantity_and_trigger_blocks() {
    let (mut p, mut i, mut m) = fixture();
    ordered(&mut p);
    i.selection.source = FactDeclaration::Unknown(text("Source missing"));
    m.source = None;
    m.source_head = None;
    let e = evaluate(p, &i, &m);
    assert_eq!(
        e.blocks(),
        &[
            DeadlineEvaluationBlock::Rule(DeadlineRuleBlock::MissingOrderedQuantity),
            DeadlineEvaluationBlock::Trigger(TriggerBlock::UnknownSource)
        ]
    );
    assert!(e.arithmetic().is_none());
}
#[test]
fn qualified_hourly_profile_uses_exact_seconds_and_utc() {
    let (mut p, mut i, m) = fixture();
    hourly(&mut p, &mut i);
    let e = evaluate(p, &i, &m);
    assert!(e.blocks().is_empty());
    assert_eq!(
        e.due_at().unwrap(),
        date("2026-01-09")
            .date()
            .with_hms(11, 30, 7)
            .unwrap()
            .assume_utc()
    );
    assert_eq!(e.due_at().unwrap().offset(), UtcOffset::UTC);
}
#[test]
fn hourly_precision_and_qualification_failures_do_not_produce_due_instants() {
    let (mut p, mut i, m) = fixture();
    hourly(&mut p, &mut i);
    i.selection.qualification.as_mut().unwrap().at = inputs::date("2026-01-06");
    let e = evaluate(p.clone(), &i, &m);
    assert!(matches!(
        e.blocks(),
        [DeadlineEvaluationBlock::Arithmetic(
            ArithmeticBlock::InsufficientPrecision { .. }
        )]
    ));
    assert!(e.due_at().is_none());
    i.selection.qualification = None;
    assert_eq!(
        evaluate(p, &i, &m).blocks(),
        &[DeadlineEvaluationBlock::Trigger(
            TriggerBlock::MissingQualification {
                purpose: QualifiedTriggerPurpose::OrderedPeriodStart
            }
        )]
    );
}
