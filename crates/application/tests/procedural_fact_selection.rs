#[path = "../../domain/tests/procedural_fact_support/mod.rs"]
mod support;

use application::procedural_facts::*;
use domain::{
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
};
use support::*;
use uuid::Uuid;

fn selected(input: NotificationValuesInput) -> FactSourceSelection {
    FactSourceSelection::from_values(&ProceduralFactValues::Notification(Box::new(
        NotificationValues::new(input).unwrap(),
    )))
}
fn participant_ref(id: u128, revision: u32) -> FactParticipantRef {
    let FactPerson::Participant(reference) = person(id, revision) else {
        panic!("participant fixture expected")
    };
    reference
}
fn hearing_ref(
    hearing: u128,
    result: u128,
    revision: u32,
    agreement: Option<u128>,
) -> FactHearingRef {
    FactHearingRef {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(hearing)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(result)),
        revision: HearingResultRevision::new(revision).unwrap(),
        agreement_id: agreement.map(|id| HearingResultAgreementId::from_uuid(Uuid::from_u128(id))),
    }
}
fn hearing(reference: FactHearingRef, support: Option<FactEvidence>) -> FactProvenance {
    FactProvenance::HearingResult {
        reference,
        locator: label("Declared agreement location"),
        support,
    }
}
fn represented(provenance: FactProvenance) -> FactRepresentation {
    FactRepresentation::Declared {
        represented: person(7, 1),
        representative: person(8, 1),
        scope: text("Declared scope"),
        provenance: Box::new(provenance),
    }
}

#[test]
fn operator_resolution_has_no_selected_sources_or_inferred_parent() {
    let values =
        ProceduralFactValues::Resolution(Box::new(ResolutionValues::new(resolution_input())));
    let sources = FactSourceSelection::from_values(&values);
    assert_eq!(sources.resolution(), None);
    assert!(sources.participants().is_empty());
    assert!(sources.hearing_results().is_empty());
    assert!(sources.direct_supports().is_empty());
}

#[test]
fn resolution_selects_only_its_declared_exact_result_and_direct_document() {
    let reference = hearing_ref(3, 4, u32::MAX, Some(5));
    let file = evidence(8, u32::MAX, 9, "p. 2");
    let mut input = resolution_input();
    input.provenance = hearing(reference, Some(file.clone()));
    let values = ProceduralFactValues::Resolution(Box::new(ResolutionValues::new(input)));
    let sources = FactSourceSelection::from_values(&values);
    assert_eq!(sources.resolution(), None);
    assert!(sources.participants().is_empty());
    assert_eq!(sources.hearing_results(), &[reference]);
    assert_eq!(sources.direct_supports(), &[file.support()]);
}

#[test]
fn unknown_and_unlinked_people_do_not_create_directory_references() {
    let mut input = notification_input();
    input.resolution.revision = FactRevision::new(7).unwrap();
    let parent = input.resolution;
    let unlinked = FactPerson::Unlinked {
        label: label("Declared name"),
        description: text("Unlinked identity"),
    };
    input.intended_recipient = FactDeclaration::Known(unlinked.clone());
    input.actual_receiver = FactDeclaration::Unknown(text("Not stated"));
    input.representation = FactRepresentation::Declared {
        represented: unlinked.clone(),
        representative: unlinked,
        scope: text("Declared scope"),
        provenance: Box::new(external(None)),
    };
    let sources = selected(input);
    assert_eq!(sources.resolution(), Some(parent));
    assert!(sources.participants().is_empty());
    assert!(sources.hearing_results().is_empty());
    assert!(sources.direct_supports().is_empty());
}

#[test]
fn all_four_person_functions_are_selected_in_uuid_then_revision_order() {
    let mut input = notification_input();
    input.intended_recipient = FactDeclaration::Known(person(9, 3));
    input.actual_receiver = FactDeclaration::Known(person(9, 1));
    input.representation = FactRepresentation::Declared {
        represented: person(0, u32::MAX),
        representative: person(4, 2),
        scope: text("Declared scope"),
        provenance: Box::new(external(None)),
    };
    let sources = selected(input);
    assert_eq!(
        sources.participants(),
        &[
            participant_ref(0, u32::MAX),
            participant_ref(4, 2),
            participant_ref(9, 1),
            participant_ref(9, 3),
        ]
    );
}

#[test]
fn repeated_exact_people_deduplicate_without_erasing_function_values() {
    let mut input = notification_input();
    input.intended_recipient = FactDeclaration::Known(person(3, 2));
    input.actual_receiver = FactDeclaration::Known(person(3, 2));
    input.representation = FactRepresentation::Declared {
        represented: person(3, 1),
        representative: person(3, 2),
        scope: text("Expressly declared relation"),
        provenance: Box::new(external(None)),
    };
    let notification = NotificationValues::new(input).unwrap();
    let original = notification.clone();
    let values = ProceduralFactValues::Notification(Box::new(notification));
    let sources = FactSourceSelection::from_values(&values);
    assert_eq!(
        sources.participants(),
        &[participant_ref(3, 1), participant_ref(3, 2)]
    );
    let ProceduralFactValues::Notification(value) = values else {
        panic!("notification fixture expected")
    };
    assert_eq!(*value, original);
}

#[test]
fn absent_representation_does_not_infer_a_relationship_from_known_people() {
    let mut input = notification_input();
    input.intended_recipient = FactDeclaration::Known(person(7, 1));
    input.actual_receiver = FactDeclaration::Known(person(8, 1));
    let sources = selected(input);
    assert_eq!(
        sources.participants(),
        &[participant_ref(7, 1), participant_ref(8, 1)]
    );
    assert!(sources.hearing_results().is_empty());
}

#[test]
fn duplicate_result_selection_deduplicates_but_retains_both_provenance_locators() {
    let reference = hearing_ref(1, 2, 3, Some(4));
    let mut input = notification_input();
    input.provenance = hearing(reference, None);
    let mut relationship = hearing(reference, None);
    if let FactProvenance::HearingResult { locator, .. } = &mut relationship {
        *locator = label("Another location for representation");
    }
    input.representation = represented(relationship);
    let original = NotificationValues::new(input).unwrap();
    let values = ProceduralFactValues::Notification(Box::new(original.clone()));
    let sources = FactSourceSelection::from_values(&values);
    assert_eq!(sources.hearing_results(), &[reference]);
    assert_eq!(
        values,
        ProceduralFactValues::Notification(Box::new(original))
    );
}

#[test]
fn result_order_preserves_hearing_result_revision_and_optional_agreement() {
    let pairs = [
        (hearing_ref(1, 9, 9, Some(9)), hearing_ref(2, 1, 1, None)),
        (hearing_ref(1, 2, 9, Some(9)), hearing_ref(1, 3, 1, None)),
        (hearing_ref(1, 2, 3, Some(9)), hearing_ref(1, 2, 4, None)),
        (hearing_ref(1, 2, 3, None), hearing_ref(1, 2, 3, Some(0))),
        (hearing_ref(1, 2, 3, Some(4)), hearing_ref(1, 2, 3, Some(5))),
    ];
    for (first, second) in pairs {
        for reverse in [false, true] {
            let mut input = notification_input();
            let (primary, relationship) = if reverse {
                (first, second)
            } else {
                (second, first)
            };
            input.provenance = hearing(primary, None);
            input.representation = represented(hearing(relationship, None));
            assert_eq!(selected(input).hearing_results(), &[first, second]);
        }
    }
}

#[test]
fn document_selection_is_ordered_by_identity_and_version_not_function() {
    for (first, second) in [
        (evidence(1, 9, 3, "p. 1"), evidence(2, 1, 4, "p. 2")),
        (evidence(1, 1, 3, "p. 1"), evidence(1, 2, 4, "p. 2")),
        (evidence(1, 1, 3, "p. 1"), evidence(2, 1, 3, "p. 2")),
        (evidence(1, 1, 3, "p. 1"), evidence(1, 2, 3, "p. 2")),
    ] {
        for reverse in [false, true] {
            let mut input = notification_input();
            let (primary, relationship) = if reverse {
                (&first, &second)
            } else {
                (&second, &first)
            };
            input.provenance = external(Some(primary.clone()));
            input.representation = represented(external(Some(relationship.clone())));
            assert_eq!(
                selected(input).direct_supports(),
                &[first.support(), second.support()]
            );
        }
    }
}

#[test]
fn shared_document_is_selected_once_while_each_function_keeps_its_locator() {
    let primary = evidence(2, 3, 4, "p. 1");
    let relationship = evidence(2, 3, 4, "p. 7");
    let mut input = notification_input();
    input.provenance = external(Some(primary.clone()));
    input.representation = represented(external(Some(relationship)));
    let original = NotificationValues::new(input).unwrap();
    let values = ProceduralFactValues::Notification(Box::new(original.clone()));
    let sources = FactSourceSelection::from_values(&values);
    assert_eq!(sources.direct_supports(), &[primary.support()]);
    assert_eq!(
        values,
        ProceduralFactValues::Notification(Box::new(original))
    );
}

#[test]
fn historical_parent_and_results_do_not_add_a_third_direct_document() {
    let first = evidence(1, 1, 3, "Primary support");
    let second = evidence(2, 1, 4, "Representation support");
    let mut input = notification_input();
    let parent = input.resolution;
    input.provenance = hearing(hearing_ref(9, 8, 7, Some(6)), Some(first.clone()));
    input.representation = represented(hearing(hearing_ref(5, 4, 3, None), Some(second.clone())));
    let sources = selected(input);
    assert_eq!(sources.resolution(), Some(parent));
    assert_eq!(sources.hearing_results().len(), 2);
    assert_eq!(
        sources.direct_supports(),
        &[first.support(), second.support()]
    );
}

#[test]
fn representation_support_is_selected_without_a_primary_document() {
    let file = evidence(0, u32::MAX, 7, "Representation support");
    let mut input = notification_input();
    input.representation = represented(external(Some(file.clone())));
    assert_eq!(selected(input).direct_supports(), &[file.support()]);
}
