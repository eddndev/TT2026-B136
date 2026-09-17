mod deadline_trigger_field_support;
mod procedural_fact_support;

use deadline_trigger_field_support::*;
use domain::{
    crypto::Sha256Digest,
    deadline_triggers::{
        extract_trigger_time, FactTriggerDigests, QualifiedTriggerPurpose, QualifiedTriggerTime,
        TriggerBlock, TriggerDigests, TriggerField, TriggerMaterial, TriggerOutcome,
        TriggerProvenance, TriggerRequirement, TriggerSelection, TriggerSourceRef,
    },
    procedural_facts::{
        FactDeclaration, FactProvenance, FactRepresentation, FactStatedEffect, NotificationValues,
        ResolutionValues,
    },
    procedural_time::DeclaredProceduralTime as Declared,
};
use procedural_fact_support::{evidence, external, label, person, resolution_input, text};
use time::UtcOffset;

#[test]
fn resolution_field_preserves_time_selection_provenance_and_each_digest() {
    let at = Declared::minute("2030-04-05".parse().unwrap(), 10, 17, None).unwrap();
    let mut input = resolution_input();
    input.issued_at = at;
    input.provenance = external(Some(evidence(31, 4, 8, "Resolution page 2")));
    let values = ResolutionValues::new(input);
    let selected = selection(TriggerSourceRef::Resolution(parent()));
    let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
    let result =
        extract_trigger_time(requirement, &selected, Some(resolution_material(&values))).unwrap();
    assert_eq!(result.requirement(), requirement);
    assert_eq!(result.selection(), &selected);
    assert_eq!(result.outcome(), &TriggerOutcome::Extracted { at });
    let source = result.source().unwrap();
    assert_eq!(source.case_id, case_id());
    assert_eq!(source.reference, TriggerSourceRef::Resolution(parent()));
    assert_eq!(source.digests, TriggerDigests::ProceduralFact(digests()));
    assert_eq!(
        source.provenance,
        TriggerProvenance::ProceduralFact(values.provenance().clone())
    );
    assert_eq!(source.agreement, None);
    assert_eq!(source.stated_effect, None);
    assert_eq!(at.local_second(), None);
    assert_eq!(at.offset(), None);
}

#[test]
fn unknown_resolution_time_is_extracted_without_substituting_another_date() {
    let values = ResolutionValues::new(resolution_input());
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selection(TriggerSourceRef::Resolution(parent())),
        Some(resolution_material(&values)),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Extracted {
            at: Declared::unknown()
        }
    );
    assert!(result.source().is_some());
}

#[test]
fn notification_fields_choose_only_the_named_time_with_its_original_precision() {
    let practiced = date("2031-01-02");
    let received = Declared::minute("2032-03-04".parse().unwrap(), 5, 6, None).unwrap();
    let effect = Declared::second(
        "2033-05-06".parse().unwrap(),
        7,
        8,
        9,
        Some(UtcOffset::from_hms(2, 30, 0).unwrap()),
    )
    .unwrap();
    let mut input = notification_input();
    input.practiced_at = practiced;
    input.received_at = Some(received);
    input.stated_effect = Some(FactStatedEffect {
        at: effect,
        statement: text("Effect expressly stated by source"),
        locator: label("Principal source paragraph 7"),
    });
    let values = NotificationValues::new(input).unwrap();
    let selected = selection(notification_ref());
    for (field, at) in [
        (TriggerField::NotificationPracticedAt, practiced),
        (TriggerField::NotificationReceivedAt, received),
        (TriggerField::NotificationStatedEffectAt, effect),
    ] {
        let requirement = TriggerRequirement::SourceField(field);
        let result =
            extract_trigger_time(requirement, &selected, Some(notification_material(&values)))
                .unwrap();
        assert_eq!(result.requirement(), requirement);
        assert_eq!(result.selection(), &selected);
        assert_eq!(result.outcome(), &TriggerOutcome::Extracted { at });
        assert_eq!(result.source().unwrap().reference, notification_ref());
    }
}

#[test]
fn optional_notification_times_absent_block_without_falling_back_to_practice() {
    let mut input = notification_input();
    input.practiced_at = date("2026-09-16");
    let values = NotificationValues::new(input).unwrap();
    for field in [
        TriggerField::NotificationReceivedAt,
        TriggerField::NotificationStatedEffectAt,
    ] {
        let result = extract_trigger_time(
            TriggerRequirement::SourceField(field),
            &selection(notification_ref()),
            Some(notification_material(&values)),
        )
        .unwrap();
        assert_eq!(
            result.outcome(),
            &TriggerOutcome::Blocked(TriggerBlock::AbsentField(field))
        );
        assert_eq!(result.source().unwrap().reference, notification_ref());
        assert_eq!(result.source().unwrap().stated_effect, None);
    }
}

#[test]
fn present_unknown_notification_times_are_not_absent_fields() {
    let mut input = notification_input();
    input.practiced_at = date("2026-09-16");
    input.received_at = Some(Declared::unknown());
    input.stated_effect = Some(FactStatedEffect {
        at: Declared::unknown(),
        statement: text("Source does not identify the effect date"),
        locator: label("Paragraph 8"),
    });
    let values = NotificationValues::new(input).unwrap();
    for field in [
        TriggerField::NotificationReceivedAt,
        TriggerField::NotificationStatedEffectAt,
    ] {
        let result = extract_trigger_time(
            TriggerRequirement::SourceField(field),
            &selection(notification_ref()),
            Some(notification_material(&values)),
        )
        .unwrap();
        assert_eq!(
            result.outcome(),
            &TriggerOutcome::Extracted {
                at: Declared::unknown()
            }
        );
        assert!(result.source().is_some());
    }
}

#[test]
fn notification_snapshot_uses_principal_provenance_not_representation_provenance() {
    let mut input = notification_input();
    input.received_at = Some(date("2026-01-05"));
    input.provenance = external(Some(evidence(31, 2, 7, "Principal page 1")));
    let principal = input.provenance.clone();
    let relationship = external(Some(evidence(32, 3, 8, "Representation page 5")));
    input.representation = FactRepresentation::Declared {
        represented: person(1, 2),
        representative: person(2, 3),
        scope: text("Relationship as declared"),
        provenance: Box::new(relationship.clone()),
    };
    let values = NotificationValues::new(input).unwrap();
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::NotificationReceivedAt),
        &selection(notification_ref()),
        Some(notification_material(&values)),
    )
    .unwrap();
    let source = result.source().unwrap();
    assert_eq!(
        source.provenance,
        TriggerProvenance::ProceduralFact(principal)
    );
    assert_ne!(
        source.provenance,
        TriggerProvenance::ProceduralFact(relationship)
    );
    assert_eq!(source.digests, TriggerDigests::ProceduralFact(digests()));
    assert_eq!(source.agreement, None);
}

#[test]
fn stated_effect_snapshot_keeps_the_statement_locator_and_primary_provenance() {
    let effect = FactStatedEffect {
        at: date("2026-02-03"),
        statement: text("Express statement\nWith its original second line"),
        locator: label("Main source section 2"),
    };
    let mut input = notification_input();
    input.stated_effect = Some(effect.clone());
    input.provenance = external(Some(evidence(34, 2, 5, "Evidence page 3")));
    let provenance = input.provenance.clone();
    let values = NotificationValues::new(input).unwrap();
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::NotificationStatedEffectAt),
        &selection(notification_ref()),
        Some(notification_material(&values)),
    )
    .unwrap();
    assert_eq!(result.source().unwrap().stated_effect, Some(effect));
    assert_eq!(
        result.source().unwrap().provenance,
        TriggerProvenance::ProceduralFact(provenance)
    );
}

#[test]
fn unconsumed_stated_effect_is_not_copied_into_another_field_snapshot() {
    let mut input = notification_input();
    input.received_at = Some(date("2026-01-04"));
    input.stated_effect = Some(FactStatedEffect {
        at: date("2026-01-07"),
        statement: text("An independent stated effect"),
        locator: label("Paragraph 5"),
    });
    let values = NotificationValues::new(input).unwrap();
    for field in [
        TriggerField::NotificationPracticedAt,
        TriggerField::NotificationReceivedAt,
    ] {
        let result = extract_trigger_time(
            TriggerRequirement::SourceField(field),
            &selection(notification_ref()),
            Some(notification_material(&values)),
        )
        .unwrap();
        assert_eq!(result.source().unwrap().stated_effect, None);
        assert!(values.stated_effect().is_some());
    }
}

#[test]
fn supplied_digest_roles_are_captured_without_claiming_hash_verification() {
    let values = ResolutionValues::new(resolution_input());
    for supplied in [
        digests(),
        FactTriggerDigests {
            values: Sha256Digest::from_array([0; 32]),
            sources: Sha256Digest::from_array([9; 32]),
            submission: Sha256Digest::from_array([4; 32]),
        },
    ] {
        let TriggerMaterial::Resolution { root, revision, .. } = resolution_material(&values)
        else {
            panic!("expected resolution material");
        };
        let result = extract_trigger_time(
            TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
            &selection(TriggerSourceRef::Resolution(parent())),
            Some(TriggerMaterial::Resolution {
                root,
                revision,
                values: &values,
                digests: supplied,
            }),
        )
        .unwrap();
        assert_eq!(
            result.source().unwrap().digests,
            TriggerDigests::ProceduralFact(supplied)
        );
    }
}

#[test]
fn later_input_replacement_cannot_rewrite_selection_provenance_or_stated_effect() {
    let mut input = notification_input();
    input.stated_effect = Some(FactStatedEffect {
        at: date("2026-01-05"),
        statement: text("Original effect"),
        locator: label("Original locator"),
    });
    input.provenance = external(Some(evidence(40, 2, 9, "Original evidence locator")));
    let mut values = NotificationValues::new(input.clone()).unwrap();
    let mut selected = selection(notification_ref());
    let original_selection = selected.clone();
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::NotificationStatedEffectAt),
        &selected,
        Some(notification_material(&values)),
    )
    .unwrap();
    let original_source = result.source().unwrap().clone();
    selected.source = FactDeclaration::Unknown(text("Later source selection"));
    selected.qualification = Some(QualifiedTriggerTime {
        purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
        at: date("2030-01-01"),
        statement: text("Later statement"),
        locator: label("Later locator"),
    });
    input.provenance = FactProvenance::OperatorNote {
        note: text("Later provenance"),
    };
    input.stated_effect = None;
    values = NotificationValues::new(input).unwrap();
    assert_ne!(selected, original_selection);
    assert_ne!(
        values.provenance(),
        match &original_source.provenance {
            TriggerProvenance::ProceduralFact(value) => value,
            _ => panic!("expected fact provenance"),
        }
    );
    assert_eq!(result.selection(), &original_selection);
    assert_eq!(result.source(), Some(&original_source));
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Extracted {
            at: date("2026-01-05")
        }
    );
    assert!(result.source().unwrap().stated_effect.is_some());
    assert!(values.stated_effect().is_none());
}

#[test]
fn unknown_source_keeps_its_reason_without_creating_a_source_snapshot() {
    let mut selected = TriggerSelection {
        case_id: case_id(),
        source: FactDeclaration::Unknown(text("Source not yet identified")),
        qualification: None,
    };
    let original = selected.clone();
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selected,
        None,
    )
    .unwrap();
    selected.source = FactDeclaration::Known(TriggerSourceRef::Resolution(parent()));
    assert_ne!(selected, original);
    assert_eq!(result.selection(), &original);
    assert_eq!(result.source(), None);
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
}
