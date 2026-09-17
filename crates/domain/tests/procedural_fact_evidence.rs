mod procedural_fact_support;
use domain::{procedural_facts::*, DomainError};
use procedural_fact_support::*;

fn with_supports(primary: FactEvidence, relationship: FactEvidence) -> NotificationValuesInput {
    let mut input = notification_input();
    input.provenance = external(Some(primary));
    input.representation = FactRepresentation::Declared {
        represented: person(1, 2),
        representative: person(2, 1),
        scope: text("Declared scope"),
        provenance: Box::new(external(Some(relationship))),
    };
    input
}

#[test]
fn two_functions_keep_their_locators_but_admit_one_exact_version() {
    let first = evidence(1, 1, 42, "Page 1");
    let second = evidence(1, 1, 42, "Page 2");
    let input = with_supports(first.clone(), second.clone());
    let values = NotificationValues::new(input.clone()).unwrap();
    let batch = values.direct_supports();
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].reference(), first.reference());
    assert_eq!(batch[0].digest(), first.digest());
    assert_eq!(
        values.provenance().support().unwrap().locator().as_str(),
        "Page 1"
    );
    assert_eq!(values.representation(), &input.representation);
}

#[test]
fn conflicting_digests_for_the_same_exact_version_are_rejected() {
    let input = with_supports(evidence(1, 1, 42, "Page 1"), evidence(1, 1, 43, "Page 2"));
    assert_eq!(
        NotificationValues::new(input),
        Err(DomainError::InvalidProceduralFact("support_digest"))
    );
}

#[test]
fn different_versions_of_one_document_are_two_admissions() {
    let input = with_supports(evidence(1, 1, 42, "Page 1"), evidence(1, 2, 43, "Page 2"));
    let values = NotificationValues::new(input).unwrap();
    let batch = values.direct_supports();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0].reference().version.get(), 1);
    assert_eq!(batch[1].reference().version.get(), 2);
}

#[test]
fn absent_primary_support_does_not_drop_relationship_support() {
    let proof = evidence(7, 3, 42, "Page 8");
    let mut input = with_supports(proof.clone(), proof.clone());
    input.provenance = external(None);
    let values = NotificationValues::new(input).unwrap();
    assert_eq!(
        values.direct_supports(),
        vec![FactSupportRef::new(proof.reference(), proof.digest())]
    );
}

#[test]
fn resolution_support_is_direct_but_its_notification_reference_is_not_a_document() {
    let proof = evidence(7, 3, 42, "Page 8");
    let mut input = resolution_input();
    input.provenance = external(Some(proof.clone()));
    assert_eq!(
        ResolutionValues::new(input).direct_supports(),
        vec![FactSupportRef::new(proof.reference(), proof.digest())]
    );
    assert!(NotificationValues::new(notification_input())
        .unwrap()
        .direct_supports()
        .is_empty());
}
