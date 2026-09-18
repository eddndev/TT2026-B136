#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_evaluation_support;
use application::{deadline_evaluations::*, deadline_profiles::*};
use deadline_evaluation_support::*;
use domain::{deadline_profiles::*, deadline_triggers::*, procedural_facts::*};

fn roundtrip(evaluation: &ProfiledDeadlineEvaluation) -> DeadlineEvaluationRecord {
    let record = DeadlineEvaluationRecord::capture(evaluation);
    let bytes = deadline_evaluation_record_bytes(&record);
    assert_eq!(&bytes[..5], b"DRES1");
    let decoded = decode_deadline_evaluation_record(&bytes).unwrap();
    assert_eq!(decoded, record);
    assert_eq!(deadline_evaluation_record_bytes(&decoded), bytes);
    decoded
}

#[test]
fn capture_preserves_civil_result_and_full_natural_trace() {
    let (p, i, m) = fixture();
    let evaluation = evaluate(p, &i, &m);
    let record = roundtrip(&evaluation);
    assert_eq!(record.requirement(), evaluation.trigger().requirement());
    assert_eq!(record.trigger_outcome(), evaluation.trigger().outcome());
    assert_eq!(record.rule(), evaluation.rule());
    assert_eq!(record.due_at(), evaluation.due_at());
    assert_eq!(record.blocks(), evaluation.blocks());
    let arithmetic = record.arithmetic().unwrap();
    assert_eq!(arithmetic.rule(), evaluation.rule().unwrap());
    assert_eq!(arithmetic.anchor(), inputs::date("2026-01-06"));
    assert_eq!(
        arithmetic.outcome(),
        evaluation.arithmetic().unwrap().outcome()
    );
    assert_eq!(
        arithmetic.trace(),
        &[DeadlineTraceRecord::NaturalDays {
            first_included: date("2026-01-06"),
            quantity: n(2),
            candidate: Some(date("2026-01-07")),
        }]
    );
}

#[test]
fn capture_keeps_hourly_trace_and_original_declaration_separate() {
    let (mut p, mut i, m) = fixture();
    hourly(&mut p, &mut i);
    let evaluation = evaluate(p, &i, &m);
    let record = roundtrip(&evaluation);
    let arithmetic = record.arithmetic().unwrap();
    assert_eq!(arithmetic.anchor().offset().unwrap().whole_seconds(), 10800);
    let DeadlineTraceRecord::ElapsedHours {
        start,
        quantity,
        candidate,
    } = &arithmetic.trace()[0]
    else {
        panic!("expected an elapsed-hours trace");
    };
    assert_eq!(start.offset(), time::UtcOffset::UTC);
    assert_eq!(*quantity, n(72));
    assert_eq!(*candidate, record.due_at());
    assert_eq!(
        candidate.unwrap().unix_timestamp() - start.unix_timestamp(),
        72 * 3600
    );
}

#[test]
fn capture_preserves_maximum_block_list_in_evaluator_order() {
    let (mut p, mut i, mut m) = fixture();
    ordered(&mut p);
    p.conditions = (0..16)
        .map(|index| DeadlineProfileCondition {
            id: id(index),
            statement: text("Declared condition"),
            reference_ids: vec![id(0)],
        })
        .collect();
    i.qualification.conditions.clear();
    i.qualification.scope_applies = FactDeclaration::Unknown(text("Scope unknown"));
    i.qualification.unresolved_incident = FactDeclaration::Unknown(text("Incident unknown"));
    i.selection.source = FactDeclaration::Unknown(text("Source unknown"));
    m.source = None;
    m.source_head = None;
    let evaluation = evaluate(p, &i, &m);
    let record = roundtrip(&evaluation);
    let mut expected = vec![
        DeadlineEvaluationBlock::ScopeUnknown,
        DeadlineEvaluationBlock::IncidentUnknown,
    ];
    expected.extend((0..16).map(|index| DeadlineEvaluationBlock::ConditionMissing(id(index))));
    expected.extend([
        DeadlineEvaluationBlock::Rule(DeadlineRuleBlock::MissingOrderedQuantity),
        DeadlineEvaluationBlock::Trigger(TriggerBlock::UnknownSource),
    ]);
    assert_eq!(record.blocks(), expected);
    assert_eq!(record.blocks().len(), 20);
    assert!(record.rule().is_none());
    assert!(record.arithmetic().is_none());
    assert!(record.due_at().is_none());
}

#[test]
fn every_truncated_prefix_and_trailing_data_are_rejected() {
    let (p, i, m) = fixture();
    let bytes =
        deadline_evaluation_record_bytes(&DeadlineEvaluationRecord::capture(&evaluate(p, &i, &m)));
    for end in 0..bytes.len() {
        assert!(
            decode_deadline_evaluation_record(&bytes[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode_deadline_evaluation_record(&extra).is_err());
    let mut prefix = bytes;
    prefix[4] = b'2';
    assert!(decode_deadline_evaluation_record(&prefix).is_err());
}
