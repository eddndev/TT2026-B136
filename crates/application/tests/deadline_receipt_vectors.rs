use application::deadlines::*;
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId, procedural_facts::FactText};
use uuid::Uuid;

#[test]
fn independent_retirement_vector_binds_actor_case_root_operation_action_base_and_state() {
    let command = DeadlineCommand {
        operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(4)),
        deadline_id: DeadlineId::from_uuid(Uuid::from_u128(3)),
        change: DeadlineChange::Retire {
            expected_revision: DeadlineRevision::new(7).unwrap(),
            reason: FactText::new("Done").unwrap(),
        },
    };
    let mut expected = b"DLTX1".to_vec();
    for id in [1_u128, 2, 3, 4] {
        expected.extend_from_slice(&id.to_be_bytes());
    }
    expected.push(3);
    expected.extend_from_slice(&7_u32.to_be_bytes());
    expected.extend_from_slice(&[0x5a; 32]);
    expected.push(1);
    expected.extend_from_slice(&4_u64.to_be_bytes());
    expected.extend_from_slice(b"Done");
    let bytes = deadline_submission_bytes(
        UserId::from_uuid(Uuid::from_u128(1)),
        CaseId::from_uuid(Uuid::from_u128(2)),
        &command,
        Sha256Digest::from_array([0x5a; 32]),
    );
    assert_eq!(bytes, expected);
    assert_eq!(bytes.len(), 119);
}

#[test]
fn reason_frames_utf8_bytes_without_normalizing_an_explicit_nil_identity() {
    let reason = "\u{1f4c4}".repeat(1000);
    let command = DeadlineCommand {
        operation_id: DeadlineOperationId::from_uuid(Uuid::nil()),
        deadline_id: DeadlineId::from_uuid(Uuid::nil()),
        change: DeadlineChange::SetAttention {
            expected_revision: DeadlineRevision::new(u32::MAX - 1).unwrap(),
            attention: DeadlineAttention::Pending,
            reason: FactText::new(&reason).unwrap(),
        },
    };
    let bytes = deadline_submission_bytes(
        UserId::from_uuid(Uuid::nil()),
        CaseId::from_uuid(Uuid::nil()),
        &command,
        Sha256Digest::from_array([0; 32]),
    );
    assert_eq!(&bytes[5..69], &[0; 64]);
    assert_eq!(bytes[69], 2);
    assert_eq!(&bytes[107..115], &4000_u64.to_be_bytes());
    assert_eq!(&bytes[115..], reason.as_bytes());
    assert_eq!(bytes.len(), 4115);
}
