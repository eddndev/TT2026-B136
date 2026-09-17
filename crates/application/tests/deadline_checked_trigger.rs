#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_checked_trigger_support;
mod deadline_input_support;

use application::{deadline_inputs::*, procedural_facts::*};
use deadline_checked_trigger_support::{inconsistent, selection};
use deadline_input_support::{date, hasher, hearing, material, notification, resolution};
use domain::{
    crypto::Sha256Digest, deadline_triggers::*, hearing_results::HearingResultAgreementId,
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

#[test]
fn exact_sources_extract_without_any_arithmetic_rule_or_quantity() {
    for source in [
        DeadlineSourceDetail::Fact(Box::new(resolution(3, true, "2026-01-06"))),
        DeadlineSourceDetail::Fact(Box::new(notification(3, true, 2, "2026-01-06"))),
        DeadlineSourceDetail::HearingResult(Box::new(hearing(3, true, "2026-01-06", &[]))),
    ] {
        let (requirement, selection) = selection(&source);
        let material = material(source);
        let before = material.clone();
        let checked = extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            None,
            &material,
        )
        .unwrap();
        let outcome: &TriggerOutcome = checked.extraction().outcome();
        let at = match outcome {
            TriggerOutcome::Extracted { at } => at,
            other => panic!("expected extraction, got {other:?}"),
        };
        assert_eq!(at.local_date(), date("2026-01-06").local_date());
        assert_eq!(checked.extraction().requirement(), requirement);
        assert_eq!(checked.extraction().selection(), &selection);
        assert_eq!(
            checked.extraction().source().unwrap().reference,
            match selection.source {
                FactDeclaration::Known(reference) => reference,
                _ => unreachable!(),
            }
        );
        assert!(checked.calendar().is_none());
        assert_eq!(material, before);
        assert!(material.administration.snapshot().is_none());
    }
}

#[test]
fn the_real_optional_field_distinguishes_absence_from_explicit_unknown() {
    for received in [
        None,
        Some(DeclaredProceduralTime::unknown()),
        Some(date("2026-02-03")),
    ] {
        let mut detail = notification(1, false, 1, "2026-01-06");
        let ProceduralFactSnapshot::Notification(snapshot) = &mut detail.snapshot else {
            unreachable!()
        };
        let mut values = deadline_input_support::notification_input(&snapshot.values);
        values.received_at = received;
        snapshot.values = NotificationValues::new(values).unwrap();
        deadline_input_support::resign_fact(&mut detail);
        let source = DeadlineSourceDetail::Fact(Box::new(detail));
        let (_, selection) = selection(&source);
        let material = material(source);
        let requirement = TriggerRequirement::SourceField(TriggerField::NotificationReceivedAt);
        let checked = extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            None,
            &material,
        )
        .unwrap();
        let expected = received.map_or(
            TriggerOutcome::Blocked(TriggerBlock::AbsentField(
                TriggerField::NotificationReceivedAt,
            )),
            |at| TriggerOutcome::Extracted { at },
        );
        assert_eq!(checked.extraction().outcome(), &expected);
        assert_eq!(checked.extraction().requirement(), requirement);
    }
}

#[test]
fn the_real_requirement_controls_semantic_blocking_after_integrity_checks() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let (_, selection) = selection(&source);
    let material = material(source);
    let requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Resolution,
    };
    let checked = extract_checked_deadline_inputs(
        hasher().as_ref(),
        requirement,
        &selection,
        None,
        &material,
    )
    .unwrap();
    assert_eq!(
        checked.extraction().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::MissingQualification {
            purpose: QualifiedTriggerPurpose::OrderedPeriodStart
        },)
    );
    assert_eq!(checked.extraction().requirement(), requirement);
    let incompatible = TriggerRequirement::SourceField(TriggerField::NotificationReceivedAt);
    let checked = extract_checked_deadline_inputs(
        hasher().as_ref(),
        incompatible,
        &selection,
        None,
        &material,
    )
    .unwrap();
    assert_eq!(
        checked.extraction().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::IncompatibleFamily {
            expected: TriggerFamily::Notification,
            actual: TriggerFamily::Resolution
        },)
    );
    assert_eq!(checked.extraction().requirement(), incompatible);
}

#[test]
fn a_semantically_blocked_requirement_cannot_hide_a_wrong_exact_revision() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let (_, mut selection) = selection(&source);
    let FactDeclaration::Known(TriggerSourceRef::Resolution(reference)) = &mut selection.source
    else {
        unreachable!()
    };
    reference.revision = FactRevision::new(2).unwrap();
    let material = material(source);
    let requirement = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        family: TriggerFamily::Resolution,
    };
    inconsistent(extract_checked_deadline_inputs(
        hasher().as_ref(),
        requirement,
        &selection,
        None,
        &material,
    ));
}

#[test]
fn exact_and_head_receipts_are_checked_before_semantic_blocking() {
    for corrupt_head in [false, true] {
        let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
        let (_, selection) = selection(&source);
        let mut material = material(source);
        material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(resolution(
            2,
            false,
            "2026-02-01",
        ))));
        let item = if corrupt_head {
            &mut material.source_head
        } else {
            &mut material.source
        };
        let Some(DeadlineSourceDetail::Fact(detail)) = item else {
            unreachable!()
        };
        deadline_input_support::metadata_mut(detail)
            .receipt
            .submission_digest = Sha256Digest::from_array([255; 32]);
        let requirement = TriggerRequirement::SourceField(TriggerField::NotificationReceivedAt);
        inconsistent(extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            None,
            &material,
        ));
    }
}

#[test]
fn a_later_head_does_not_replace_the_exact_source_time() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let (requirement, selection) = selection(&source);
    let mut material = material(source);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(resolution(
        3,
        true,
        "2026-02-03",
    ))));
    let checked = extract_checked_deadline_inputs(
        hasher().as_ref(),
        requirement,
        &selection,
        None,
        &material,
    )
    .unwrap();
    assert_eq!(
        checked.extraction().outcome(),
        &TriggerOutcome::Extracted {
            at: date("2026-01-06")
        }
    );
}

#[test]
fn notification_parent_and_hearing_agreement_are_exact_even_when_qualification_is_missing() {
    for source in [
        DeadlineSourceDetail::Fact(Box::new(notification(1, false, 1, "2026-01-06"))),
        DeadlineSourceDetail::HearingResult(Box::new(hearing(1, false, "2026-01-06", &[]))),
    ] {
        let (_, mut selection) = selection(&source);
        let family = match &mut selection.source {
            FactDeclaration::Known(TriggerSourceRef::Notification { resolution, .. }) => {
                resolution.revision = FactRevision::new(2).unwrap();
                TriggerFamily::Notification
            }
            FactDeclaration::Known(TriggerSourceRef::HearingResult(reference)) => {
                reference.agreement_id = Some(HearingResultAgreementId::from_uuid(Uuid::nil()));
                TriggerFamily::HearingResult
            }
            _ => unreachable!(),
        };
        let requirement = TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
            family,
        };
        let material = material(source);
        inconsistent(extract_checked_deadline_inputs(
            hasher().as_ref(),
            requirement,
            &selection,
            None,
            &material,
        ));
    }
}
