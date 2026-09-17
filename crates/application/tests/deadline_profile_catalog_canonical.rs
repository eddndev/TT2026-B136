#[allow(dead_code)]
mod case_support;
mod deadline_profile_catalog_support;
use application::deadline_profiles::*;
use deadline_profile_catalog_support::*;
use domain::{crypto::Sha256Digest, identity::UserId};
use uuid::Uuid;

#[test]
fn publication_receipt_has_independent_fixed_wire_bytes_including_algorithm() {
    let command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::from_uuid(Uuid::from_bytes([0x22; 16])),
        profile_id: DeadlineProfileId::from_uuid(Uuid::from_bytes([0x33; 16])),
        change: DeadlineProfileChange::Publish {
            definition: definition(None),
        },
    };
    let bytes = deadline_profile_submission_bytes(
        UserId::from_uuid(Uuid::from_bytes([0x11; 16])),
        &command,
        DeadlineProfileAlgorithm::V1,
        Sha256Digest::from_array([0x44; 32]),
    );
    let expected = concat!(
        "4450545831",
        "11111111111111111111111111111111",
        "22222222222222222222222222222222",
        "33333333333333333333333333333333",
        "00",
        "00000000",
        "01",
        "4444444444444444444444444444444444444444444444444444444444444444",
        "00",
    );
    assert_eq!(bytes.len(), 92);
    let actual = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(actual, expected);
}

#[test]
fn definition_digest_uses_dprf_bytes_and_reason_and_operation_change_the_receipt() {
    use domain::crypto::DocumentHasher;
    let actor = UserId::new();
    let first = command();
    let values = definition(None);
    assert_eq!(
        deadline_profile_definition_digest(&Hasher, &values),
        Hasher.hash_bytes(&deadline_profile_definition_bytes(&values))
    );
    let base = detail(actor, &first, values);
    let mut replacement = replacement(&base);
    let original = deadline_profile_submission_bytes(
        actor,
        &replacement,
        base.algorithm,
        base.definition_digest,
    );
    replacement.operation_id = DeadlineProfileOperationId::new();
    assert_ne!(
        deadline_profile_submission_bytes(
            actor,
            &replacement,
            base.algorithm,
            base.definition_digest
        ),
        original
    );
    if let DeadlineProfileChange::Replace { reason, .. } = &mut replacement.change {
        *reason = text("Changed stated reason");
    }
    let changed = deadline_profile_submission_bytes(
        actor,
        &replacement,
        base.algorithm,
        base.definition_digest,
    );
    assert!(changed.ends_with(b"Changed stated reason"));
    assert_ne!(changed, original);
}
