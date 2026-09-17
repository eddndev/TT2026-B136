#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_input_support;
use application::{
    deadline_inputs::*, hearing_results::*, judicial_calendars::*, procedural_facts::*,
};
use deadline_input_support::*;
use domain::{
    cases::CaseId, crypto::Sha256Digest, deadline_arithmetic::ArithmeticOutcome,
    deadline_triggers::*,
};

#[test]
fn exact_resolution_yields_its_declared_date_and_preserves_unrevised_context() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2035-01-06")));
    let request = request(&source);
    let material = material(source);
    let before = material.clone();
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Extracted {
            at: date("2035-01-06")
        }
    );
    assert_eq!(
        result.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2035-01-06".parse().unwrap()
        }
    );
    assert_eq!(result.trigger().selection(), &request.trigger);
    assert_eq!(material, before);
    assert!(material.administration.snapshot().is_none());
}

#[test]
fn unknown_source_remains_blocked_with_no_manufactured_material() {
    let (request, material) = unknown();
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result.trigger().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
    assert!(result.trigger().source().is_none());
    assert!(result.arithmetic().is_none());
}

#[test]
fn source_and_head_presence_must_exactly_match_known_or_unknown_selection() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let request = request(&source);
    let base = material(source.clone());
    for (exact, head) in [
        (None, base.source_head.clone()),
        (base.source.clone(), None),
        (None, None),
    ] {
        let mut bad = base.clone();
        bad.source = exact;
        bad.source_head = head;
        inconsistent(check_deadline_inputs(hasher().as_ref(), &request, &bad));
    }
    let (unknown_request, mut unknown_material) = unknown();
    for (exact, head) in [
        (Some(source.clone()), None),
        (None, Some(source.clone())),
        (Some(source.clone()), Some(source)),
    ] {
        unknown_material.source = exact;
        unknown_material.source_head = head;
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &unknown_request,
            &unknown_material,
        ));
    }
}

#[test]
fn mismatched_material_case_is_not_hidden_by_unknown_source() {
    let (request, mut material) = unknown();
    material.case_id = CaseId::new();
    inconsistent(check_deadline_inputs(
        hasher().as_ref(),
        &request,
        &material,
    ));
}

#[test]
fn exact_fact_corrupt_values_sources_or_submission_are_rejected() {
    for kind in 0..3 {
        let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
        let request = request(&source);
        let mut material = material(source);
        let Some(DeadlineSourceDetail::Fact(detail)) = &mut material.source else {
            unreachable!()
        };
        let metadata = metadata_mut(detail);
        let corrupted = Sha256Digest::from_array([255; 32]);
        match kind {
            0 => metadata.values_digest = corrupted,
            1 => metadata.receipt.sources_digest = corrupted,
            _ => metadata.receipt.submission_digest = corrupted,
        }
        material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(resolution(
            2,
            false,
            "2026-01-06",
        ))));
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}

#[test]
fn all_source_families_extract_the_selected_field_without_inferring_eligibility() {
    let sources = [
        DeadlineSourceDetail::Fact(Box::new(resolution(3, true, "2026-01-06"))),
        DeadlineSourceDetail::Fact(Box::new(notification(3, true, 2, "2026-01-06"))),
        DeadlineSourceDetail::HearingResult(Box::new(hearing(3, true, "2026-01-06", &[]))),
    ];
    for source in sources {
        let request = request(&source);
        let material = material(source);
        let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
        assert_eq!(
            result.arithmetic().unwrap().outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: "2026-01-06".parse().unwrap()
            }
        );
        assert_eq!(
            result.trigger().source().unwrap().reference,
            match request.trigger.source {
                FactDeclaration::Known(r) => r,
                _ => unreachable!(),
            }
        );
    }
}

#[test]
fn values_and_readable_source_projections_cannot_change_without_their_receipt() {
    let mut fact = notification(1, false, 1, "2026-01-06");
    let request = request(&DeadlineSourceDetail::Fact(Box::new(fact.clone())));
    fact.sources.views.resolution.as_mut().unwrap().summary =
        FactText::new("Altered parent projection").unwrap();
    let bad = material(DeadlineSourceDetail::Fact(Box::new(fact)));
    inconsistent(check_deadline_inputs(hasher().as_ref(), &request, &bad));
    let mut hearing = hearing(1, false, "2026-01-06", &[]);
    let request = deadline_input_support::request(&DeadlineSourceDetail::HearingResult(Box::new(
        hearing.clone(),
    )));
    hearing.anchor.reference.values_digest = Sha256Digest::from_array([255; 32]);
    let bad = material(DeadlineSourceDetail::HearingResult(Box::new(hearing)));
    inconsistent(check_deadline_inputs(hasher().as_ref(), &request, &bad));
}

#[path = "deadline_input_support/administration_cases.rs"]
mod administration_cases;
#[path = "deadline_input_support/calendar_cases.rs"]
mod calendar_cases;

#[test]
fn changed_value_with_unchanged_digest_is_rejected_even_when_exact_and_head_agree() {
    let mut detail = resolution(1, false, "2026-01-06");
    let request = request(&DeadlineSourceDetail::Fact(Box::new(detail.clone())));
    let ProceduralFactSnapshot::Resolution(snapshot) = &mut detail.snapshot else {
        unreachable!()
    };
    let original = &snapshot.values;
    snapshot.values = ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: original.subtype().cloned(),
        issuer: original.issuer().clone(),
        issued_at: date("2026-12-31"),
        summary: original.summary().clone(),
        provenance: original.provenance().clone(),
    });
    let material = material(DeadlineSourceDetail::Fact(Box::new(detail)));
    inconsistent(check_deadline_inputs(
        hasher().as_ref(),
        &request,
        &material,
    ));
}

#[path = "deadline_input_support/material_identity_cases.rs"]
mod material_identity_cases;
