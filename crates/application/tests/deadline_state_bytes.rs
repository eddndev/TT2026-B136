#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
use application::{cases::CurrentCaseAdministration, deadlines::*};
use deadline_support::{evaluation::inputs, *};
use domain::cases::CaseMetadata;

#[test]
fn storage_preimages_match_the_existing_review_and_capture_receipts() {
    let (command, preparation) = fixture();
    let value = detail(&prepare(command, preparation).unwrap());
    let hasher = inputs::hasher();
    let review = deadline_review_bytes(hasher.as_ref(), &value).unwrap();
    let capture = deadline_capture_bytes(hasher.as_ref(), &value).unwrap();
    assert_eq!(&review[..5], b"DLRV1");
    assert_eq!(&capture[..5], b"DLST1");
    assert_eq!(hasher.hash_bytes(&review), value.receipt.review_digest);
    assert_eq!(hasher.hash_bytes(&capture), value.receipt.capture_digest);
    assert_ne!(review, capture);
}

#[test]
fn observed_administration_changes_only_the_capture_preimage() {
    let (command, preparation) = fixture();
    let before = detail(&prepare(command, preparation).unwrap());
    let mut after = before.clone();
    after.calculation.material.administration = CurrentCaseAdministration::Unrevised(
        CaseMetadata::new("A later observed title", "REF-2").unwrap(),
    );
    let hasher = inputs::hasher();
    assert_eq!(
        deadline_review_bytes(hasher.as_ref(), &before).unwrap(),
        deadline_review_bytes(hasher.as_ref(), &after).unwrap()
    );
    assert_ne!(
        deadline_capture_bytes(hasher.as_ref(), &before).unwrap(),
        deadline_capture_bytes(hasher.as_ref(), &after).unwrap()
    );
}

#[test]
fn captured_responsible_and_attention_are_in_both_preimages() {
    let (command, preparation) = fixture();
    let before = detail(&prepare(command, preparation).unwrap());
    let hasher = inputs::hasher();
    for change in 0..2 {
        let mut after = before.clone();
        if change == 0 {
            after.responsible.email = "changed@example.test".into();
        } else {
            after.attention = attention();
        }
        assert_ne!(
            deadline_review_bytes(hasher.as_ref(), &before).unwrap(),
            deadline_review_bytes(hasher.as_ref(), &after).unwrap()
        );
        assert_ne!(
            deadline_capture_bytes(hasher.as_ref(), &before).unwrap(),
            deadline_capture_bytes(hasher.as_ref(), &after).unwrap()
        );
    }
}
