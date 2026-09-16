mod procedural_fact_support;
use domain::{
    cases::CaseId,
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    procedural_facts::*,
    procedural_time::DeclaredProceduralTime,
    DomainError,
};
use procedural_fact_support::*;
use uuid::Uuid;

#[test]
fn resolution_preserves_declared_fields_without_requiring_a_hearing_or_document() {
    let input = resolution_input();
    let values = ResolutionValues::new(input.clone());
    assert_eq!(values.class(), &input.class);
    assert_eq!(values.subtype(), None);
    assert_eq!(values.issuer(), &input.issuer);
    assert_eq!(values.issued_at(), input.issued_at);
    assert_eq!(values.summary(), &input.summary);
    assert_eq!(values.provenance(), &input.provenance);
    assert!(values.direct_supports().is_empty());
}

#[test]
fn publication_preserves_its_electronic_medium_and_independent_context() {
    let mut input = notification_input();
    input.character = FactDeclaration::Known(NotificationCharacter::Publication);
    input.context = FactDeclaration::Known(NotificationContext::OutsideHearing);
    input.subtype = Some(label("Declared subtype"));
    let values = NotificationValues::new(input.clone()).unwrap();
    assert_eq!(values.character(), &input.character);
    assert_eq!(values.medium(), &input.medium);
    assert_eq!(values.context(), &input.context);
    assert_eq!(values.outcome(), &input.outcome);
    assert_eq!(values.subtype(), input.subtype.as_ref());
    assert_eq!(values.summary(), &input.summary);
    assert_eq!(values.provenance(), &input.provenance);
}

#[test]
fn practice_receipt_and_stated_effect_do_not_fill_each_other() {
    let mut input = notification_input();
    input.practiced_at = DeclaredProceduralTime::date("2026-09-16".parse().unwrap(), None).unwrap();
    let values = NotificationValues::new(input.clone()).unwrap();
    assert_eq!(values.practiced_at(), input.practiced_at);
    assert_eq!(values.received_at(), None);
    assert_eq!(values.stated_effect(), None);
    input.received_at = Some(DeclaredProceduralTime::unknown());
    input.stated_effect = Some(FactStatedEffect {
        at: DeclaredProceduralTime::unknown(),
        statement: text("Effect time is not specified in the referenced statement"),
        locator: label("Paragraph 3"),
    });
    let values = NotificationValues::new(input.clone()).unwrap();
    assert_eq!(values.received_at(), input.received_at);
    assert_eq!(values.stated_effect(), input.stated_effect.as_ref());
}

#[test]
fn same_person_in_two_functions_does_not_create_representation() {
    let mut input = notification_input();
    input.actual_receiver = input.intended_recipient.clone();
    let values = NotificationValues::new(input.clone()).unwrap();
    assert_eq!(values.intended_recipient(), values.actual_receiver());
    assert_eq!(values.representation(), &input.representation);
    assert!(values.direct_supports().is_empty());
}

#[test]
fn unlinked_person_and_express_representation_preserve_both_endpoints() {
    let mut input = notification_input();
    let represented = FactPerson::Unlinked {
        label: label("Person declared in source"),
        description: text("No directory selection has been made"),
    };
    input.intended_recipient = FactDeclaration::Known(represented.clone());
    input.representation = FactRepresentation::Declared {
        represented,
        representative: person(2, 5),
        scope: text("Scope stated in the source"),
        provenance: Box::new(external(None)),
    };
    let values = NotificationValues::new(input.clone()).unwrap();
    assert_eq!(values.intended_recipient(), &input.intended_recipient);
    assert_eq!(values.actual_receiver(), &input.actual_receiver);
    assert_eq!(values.representation(), &input.representation);
}

#[test]
fn hearing_origin_preserves_exact_result_and_optional_agreement() {
    let reference = FactHearingRef {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(10)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(11)),
        revision: HearingResultRevision::new(4).unwrap(),
        agreement_id: Some(HearingResultAgreementId::from_uuid(Uuid::from_u128(12))),
    };
    let mut input = resolution_input();
    input.provenance = FactProvenance::HearingResult {
        reference,
        locator: label("Agreement text"),
        support: None,
    };
    let values = ResolutionValues::new(input.clone());
    assert_eq!(values.provenance(), &input.provenance);
    assert_eq!(values.provenance().hearing_reference(), Some(reference));
    assert!(values.direct_supports().is_empty());
}

#[test]
fn distinct_notification_roots_allow_identical_values_for_one_resolution() {
    let case = CaseId::from_uuid(Uuid::from_u128(9));
    let values = NotificationValues::new(notification_input()).unwrap();
    let first = NotificationRoot::new(
        NotificationId::from_uuid(Uuid::from_u128(1)),
        case,
        values.resolution().id,
    );
    let second = NotificationRoot::new(
        NotificationId::from_uuid(Uuid::from_u128(2)),
        case,
        values.resolution().id,
    );
    assert_ne!(first.id(), second.id());
    assert_eq!(first.case_id(), second.case_id());
    assert_eq!(first.resolution_id(), second.resolution_id());
    assert!(first.validate_values(&values).is_ok());
    assert!(second.validate_values(&values).is_ok());
}

#[test]
fn correction_can_select_another_revision_but_cannot_change_resolution_root() {
    let mut input = notification_input();
    let root = NotificationRoot::new(NotificationId::new(), CaseId::new(), input.resolution.id);
    input.resolution.revision = FactRevision::new(7).unwrap();
    input.intended_recipient = FactDeclaration::Known(person(3, 9));
    assert!(root
        .validate_values(&NotificationValues::new(input.clone()).unwrap())
        .is_ok());
    input.resolution.id = ResolutionId::new();
    assert_eq!(
        root.validate_values(&NotificationValues::new(input).unwrap()),
        Err(DomainError::InvalidProceduralFact("resolution_root"))
    );
}

#[test]
fn resolution_root_preserves_identity_and_case_without_requiring_any_notification() {
    let id = ResolutionId::new();
    let case = CaseId::new();
    let root = ResolutionRoot::new(id, case);
    assert_eq!(root.id(), id);
    assert_eq!(root.case_id(), case);
    assert_ne!(root, ResolutionRoot::new(id, CaseId::new()));
}
