#[path = "../../domain/tests/typed_participant_vectors_support/mod.rs"]
mod values;
use application::typed_participants::*;
use domain::{
    cases::CaseId,
    crypto::{CredentialTrustInspection, DocumentHasher, Sha256Digest},
};
use infrastructure::RingSha256Hasher;
#[test]
fn independent_declaration_vectors_match_for_kept_and_appended_subjects() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/typed_participant_vectors.json"
    ))
    .unwrap();
    let subject = values::subject(&rows[0]["input"]);
    let role = rows
        .iter()
        .find(|v| v["name"].as_str().unwrap() == "role_control_judge")
        .unwrap();
    let role = values::participant(&role["input"]);
    for vector in rows
        .iter()
        .filter(|v| v["name"].as_str().unwrap().starts_with("credential_"))
    {
        let input = &vector["input"];
        let bound = SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(
                Uuid::parse_str(input["subject_id"].as_str().unwrap()).unwrap(),
            ),
            revision: SubjectRevision::new(
                input["proposed_subject_revision"].as_u64().unwrap() as u32
            )
            .unwrap(),
            values_digest: Sha256Digest::from_hex(input["subject_digest"].as_str().unwrap())
                .unwrap(),
        };
        let change = if input["subject_operation"] == 0 {
            SubjectChange::Keep(bound)
        } else {
            SubjectChange::Append {
                id: bound.id,
                expected: SubjectExpectation::Absent,
                values: subject.clone(),
            }
        };
        let expected = if input["expected_participant_revision"] == 0 {
            ParticipantExpectation::Absent
        } else {
            ParticipantExpectation::Revision(
                ParticipantRevision::new(
                    input["expected_participant_revision"].as_u64().unwrap() as u32
                )
                .unwrap(),
            )
        };
        let proposal = ParticipantProposal::new(
            change,
            ParticipantId::from_uuid(
                Uuid::parse_str(input["participant_id"].as_str().unwrap()).unwrap(),
            ),
            expected,
            TypedParticipantValues::new(bound, DirectoryStatus::Active, role.role().clone()),
        )
        .unwrap();
        let trust = CredentialTrustSnapshot {
            deployment_id: Uuid::parse_str(input["deployment_id"].as_str().unwrap()).unwrap(),
            revision: CredentialTrustRevision::initial(),
            inspection: CredentialTrustInspection {
                root_der: vec![],
                crl_der: vec![],
                root_fingerprint: Sha256Digest::from_hex(
                    input["root_fingerprint"].as_str().unwrap(),
                )
                .unwrap(),
                crl_digest: Sha256Digest::from_array([0; 32]),
                crl_number: 1,
                crl_this_update: 0,
                crl_next_update: 1,
                valid_from: 0,
                valid_until: 1,
            },
            published_at: time::OffsetDateTime::UNIX_EPOCH,
            published_by: "test".into(),
        };
        let bytes = participant_declaration_bytes(
            CaseId::from_uuid(Uuid::parse_str(input["case_id"].as_str().unwrap()).unwrap()),
            &proposal,
            &trust,
            Sha256Digest::from_hex(input["certificate_fingerprint"].as_str().unwrap()).unwrap(),
            &RingSha256Hasher,
        );
        assert_eq!(bytes.len(), 218);
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            vector["hex"].as_str().unwrap()
        );
        assert_eq!(
            RingSha256Hasher.hash_bytes(&bytes).to_hex(),
            vector["sha256"].as_str().unwrap()
        );
    }
}
