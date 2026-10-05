use crate::replacement_support::*;
use application::precautionary_hearings::PrecautionaryContextExpectation;
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    crypto::Sha256Digest,
    identity::{Role, UserId},
    typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef},
};

// Independently packed MATXN1: actor2/email a@b/Litigator, case1/op500,
// prior20/R7/digest33, contextR3/R2/digest44, reason R, tag2, new10,
// subject90/R3/digest55. Digest labels are repeated hexadecimal bytes.
const EXPECTED: &str = concat!(
    "4d4154584e31000000000000000000000000000000020000000361406201000000000000000000000000000000010000",
    "00000000000000000000000001f400000000000000000000000000000014000000073333333333333333333333333333",
    "333333333333333333333333333333333333000000030000000244444444444444444444444444444444444444444444",
    "444444444444444444440000000152020000000000000000000000000000000a0000000000000000000000000000005a",
    "000000035555555555555555555555555555555555555555555555555555555555555555",
);

#[test]
fn replacement_instruction_matches_independent_literal_action_two_vector() {
    let actor = Principal {
        id: UserId::from_uuid(uuid::Uuid::from_u128(2)),
        email: "a@b".into(),
        role: Role::Litigator,
    };
    let command = MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(500)),
        target: PrecautionaryMeasureRef::new(
            id(20),
            MeasureRevision::new(7).unwrap(),
            Sha256Digest::from_array([0x33; 32]),
        ),
        context: PrecautionaryContextExpectation {
            administration_revision: CaseRevision::new(3).unwrap(),
            stage_revision: CaseStageRevision::new(2).unwrap(),
            context_digest: Sha256Digest::from_array([0x44; 32]),
        },
        reason: note("R"),
        action: MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id: id(10),
            subject: SubjectRevisionRef {
                id: CaseSubjectId::from_uuid(uuid::Uuid::from_u128(90)),
                revision: SubjectRevision::new(3).unwrap(),
                values_digest: Sha256Digest::from_array([0x55; 32]),
            },
        },
    };
    let bytes = measure_administrative_submission_bytes(
        &actor,
        CaseId::from_uuid(uuid::Uuid::from_u128(1)),
        &command,
    )
    .unwrap();
    assert_eq!(bytes.len(), 228);
    let actual = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(actual, EXPECTED);
}
