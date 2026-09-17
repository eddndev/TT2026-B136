mod deadline_trigger_integrity_support;
mod hearing_result_support;
mod procedural_fact_support;

use deadline_trigger_integrity_support::*;
use domain::{deadline_triggers::*, hearing_results::HearingResultValues, procedural_facts::*};
use procedural_fact_support::{notification_input, resolution_input, text};
use uuid::Uuid;

fn notification_values() -> NotificationValues {
    let mut input = notification_input();
    input.resolution = resolution();
    NotificationValues::new(input).unwrap()
}

#[test]
fn unknown_source_without_material_retains_its_reason_and_has_no_snapshot() {
    let selected = TriggerSelection {
        case_id: case(1),
        source: FactDeclaration::Unknown(text("No source has been selected")),
        qualification: None,
    };
    let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
    let result = extract_trigger_time(requirement, &selected, None).unwrap();
    assert_eq!(result.requirement(), requirement);
    assert_eq!(result.selection(), &selected);
    assert_eq!(result.source(), None);
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
}

#[test]
fn unknown_source_rejects_material_of_every_family_instead_of_using_it() {
    let resolution = ResolutionValues::new(resolution_input());
    let notification = notification_values();
    let hearing = HearingResultValues::new(hearing_result_support::input()).unwrap();
    let selected = TriggerSelection {
        case_id: case(1),
        source: FactDeclaration::Unknown(text("Reference not identified")),
        qualification: None,
    };
    for material in [
        resolution_material(&resolution),
        notification_material(&notification),
        hearing_material(&hearing),
    ] {
        reject(
            TriggerField::ResolutionIssuedAt,
            &selected,
            Some(material),
            TriggerIntegrityError::UnexpectedMaterial,
        );
    }
}

#[test]
fn every_known_reference_requires_its_own_material() {
    for source in [
        TriggerSourceRef::Resolution(resolution()),
        notice_ref(),
        TriggerSourceRef::HearingResult(hearing()),
    ] {
        reject(
            TriggerField::ResolutionIssuedAt,
            &selection(source),
            None,
            TriggerIntegrityError::MissingMaterial,
        );
    }
}

#[test]
fn material_family_must_match_selection_before_requirement_compatibility() {
    let resolution = ResolutionValues::new(resolution_input());
    let notification = notification_values();
    let hearing = HearingResultValues::new(hearing_result_support::input()).unwrap();
    let sources = [
        TriggerSourceRef::Resolution(self::resolution()),
        notice_ref(),
        TriggerSourceRef::HearingResult(self::hearing()),
    ];
    let materials = [
        resolution_material(&resolution),
        notification_material(&notification),
        hearing_material(&hearing),
    ];
    for (index, source) in sources.into_iter().enumerate() {
        for (other, material) in materials.into_iter().enumerate() {
            if index != other {
                reject(
                    TriggerField::ResolutionIssuedAt,
                    &selection(source),
                    Some(material),
                    TriggerIntegrityError::SourceMismatch,
                );
            }
        }
    }
}

#[test]
fn resolution_material_requires_exact_case_identity_and_revision() {
    let values = ResolutionValues::new(resolution_input());
    let selected = selection(TriggerSourceRef::Resolution(resolution()));
    for (root, revision, expected) in [
        (
            ResolutionRoot::new(resolution().id, case(2)),
            resolution().revision,
            TriggerIntegrityError::CaseMismatch,
        ),
        (
            ResolutionRoot::new(ResolutionId::from_uuid(Uuid::from_u128(101)), case(1)),
            resolution().revision,
            TriggerIntegrityError::SourceMismatch,
        ),
        (
            ResolutionRoot::new(resolution().id, case(1)),
            FactRevision::new(2).unwrap(),
            TriggerIntegrityError::SourceMismatch,
        ),
    ] {
        let material = TriggerMaterial::Resolution {
            root,
            revision,
            values: &values,
            digests: fact_digests(),
        };
        reject(
            TriggerField::NotificationPracticedAt,
            &selected,
            Some(material),
            expected,
        );
    }
}

#[test]
fn notification_material_requires_exact_case_identity_and_revision() {
    let values = notification_values();
    let selected = selection(notice_ref());
    for (root, revision, expected) in [
        (
            NotificationRoot::new(notification_id(), case(2), resolution().id),
            FactRevision::new(4).unwrap(),
            TriggerIntegrityError::CaseMismatch,
        ),
        (
            NotificationRoot::new(
                NotificationId::from_uuid(Uuid::from_u128(201)),
                case(1),
                resolution().id,
            ),
            FactRevision::new(4).unwrap(),
            TriggerIntegrityError::SourceMismatch,
        ),
        (
            NotificationRoot::new(notification_id(), case(1), resolution().id),
            FactRevision::new(5).unwrap(),
            TriggerIntegrityError::SourceMismatch,
        ),
    ] {
        let material = TriggerMaterial::Notification {
            root,
            revision,
            values: &values,
            digests: fact_digests(),
        };
        reject(
            TriggerField::ResolutionIssuedAt,
            &selected,
            Some(material),
            expected,
        );
    }
}

#[test]
fn notification_root_must_match_values_even_when_selection_matches_values() {
    let mut input = notification_input();
    input.resolution = FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::from_u128(101)),
        revision: resolution().revision,
    };
    let selected = selection(TriggerSourceRef::Notification {
        id: notification_id(),
        revision: FactRevision::new(4).unwrap(),
        resolution: input.resolution,
    });
    let values = NotificationValues::new(input).unwrap();
    reject(
        TriggerField::ResolutionIssuedAt,
        &selected,
        Some(notification_material(&values)),
        TriggerIntegrityError::ParentMismatch,
    );
}

#[test]
fn notification_root_parent_cannot_be_substituted_while_values_remain_exact() {
    let values = notification_values();
    let material = TriggerMaterial::Notification {
        root: NotificationRoot::new(
            notification_id(),
            case(1),
            ResolutionId::from_uuid(Uuid::from_u128(101)),
        ),
        revision: FactRevision::new(4).unwrap(),
        values: &values,
        digests: fact_digests(),
    };
    reject(
        TriggerField::ResolutionIssuedAt,
        &selection(notice_ref()),
        Some(material),
        TriggerIntegrityError::ParentMismatch,
    );
}

#[test]
fn notification_selection_requires_the_exact_parent_id_and_revision() {
    let values = notification_values();
    for parent in [
        FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(101)),
            revision: resolution().revision,
        },
        FactResolutionRef {
            id: resolution().id,
            revision: FactRevision::new(2).unwrap(),
        },
    ] {
        let selected = selection(TriggerSourceRef::Notification {
            id: notification_id(),
            revision: FactRevision::new(4).unwrap(),
            resolution: parent,
        });
        reject(
            TriggerField::ResolutionIssuedAt,
            &selected,
            Some(notification_material(&values)),
            TriggerIntegrityError::ParentMismatch,
        );
    }
}

#[test]
fn matching_notification_parent_needs_no_synthetic_resolution_material() {
    let values = notification_values();
    let selected = selection(notice_ref());
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt),
        &selected,
        Some(notification_material(&values)),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Extracted {
            at: values.practiced_at()
        }
    );
    let source = result.source().unwrap();
    assert_eq!(source.case_id, case(1));
    assert_eq!(source.reference, notice_ref());
    assert_eq!(
        source.digests,
        TriggerDigests::ProceduralFact(fact_digests())
    );
    assert_eq!(
        source.provenance,
        TriggerProvenance::ProceduralFact(values.provenance().clone())
    );
    assert_eq!(source.agreement, None);
    assert_eq!(source.stated_effect, None);
}

#[test]
fn zero_uuid_is_an_exact_resolution_identity_not_an_absent_source() {
    let values = ResolutionValues::new(resolution_input());
    let reference = FactResolutionRef {
        id: ResolutionId::from_uuid(Uuid::nil()),
        revision: FactRevision::initial(),
    };
    let selected = selection(TriggerSourceRef::Resolution(reference));
    let result = extract_trigger_time(
        TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt),
        &selected,
        Some(TriggerMaterial::Resolution {
            root: ResolutionRoot::new(reference.id, case(1)),
            revision: reference.revision,
            values: &values,
            digests: fact_digests(),
        }),
    )
    .unwrap();
    assert_eq!(
        result.source().unwrap().reference,
        TriggerSourceRef::Resolution(reference)
    );
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Extracted {
            at: values.issued_at()
        }
    );
}
