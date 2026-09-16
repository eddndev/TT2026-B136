mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod declaration_fixture;
mod typed_participant_service_support;
use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustStore},
    typed_participants::*,
};
use domain::crypto::{InternalDeclarationVerifier, Signature};
use infrastructure::{certificates::InternalRsaDeclarationVerifier, PostgresCredentialTrustStore};
use std::sync::Arc;
use typed_participant_service_support::*;

#[test]
fn personal_openssl_signature_binds_exact_proposal_and_is_preserved_on_status_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let fx = declaration_fixture::fixture();
    db.at = time::OffsetDateTime::from_unix_timestamp(fx.at)
        .unwrap()
        .replace_nanosecond(123456789)
        .unwrap();
    let trust = InternalRsaDeclarationVerifier
        .inspect_trust(&fx.root, &fx.crl, fx.at)
        .unwrap();
    PostgresCredentialTrustStore::open(&db.admin_url, Arc::new(FixedClock(db.at)))
        .unwrap()
        .publish(CredentialTrustExpectation::Absent, trust)
        .unwrap();
    let record = upload(&db, db.case, "identity.pdf");
    let workflow = service(&db, FormatCheck(None));
    let mut value = proposal(&record);
    value.certificate = Some(fx.leaf.clone());
    value.role = ParticipantRoleValues::new(
        None,
        None,
        ParticipantProfile::ControlJudge(ControlJudgeProfile::new("Declared court").unwrap()),
        value.role.role_support().clone(),
    )
    .unwrap();
    let mut request = reviewed(
        workflow
            .review_participant("session", db.case, value)
            .unwrap(),
    );
    request.certificate = Some(fx.leaf.clone());
    let draft = workflow
        .prepare_participant("session", db.case, request.clone())
        .unwrap();
    assert!(draft.submission_digest.is_none());
    let declaration = draft.declaration.unwrap();
    assert_eq!(declaration.bytes.len(), 218);
    let directory = tempfile::tempdir().unwrap();
    let statement = directory.path().join("statement.bin");
    std::fs::write(&statement, &declaration.bytes).unwrap();
    let signature = Signature::from_bytes(declaration_fixture::openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        fx.ca
            .join("private/synthetic-declarant.key.pem")
            .to_str()
            .unwrap(),
        statement.to_str().unwrap(),
    ]))
    .unwrap();
    let committed = workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: Some(signature.clone()),
            },
        )
        .unwrap();
    let ParticipantRevisionSnapshot::Typed(ref typed) = committed.revision else {
        panic!()
    };
    let evidence = store(&db)
        .credential(db.owner, db.case, typed.id, typed.revision, db.at)
        .unwrap();
    assert_eq!(evidence.declaration, declaration.bytes);
    assert_eq!(evidence.check.signature, signature);
    assert_eq!(evidence.accepted_at, db.at);
    assert_eq!(evidence.check.checked_at, fx.at);
    assert_eq!(evidence.accepted_by.email, "owner@example.test");
    use application::participants::ParticipantStore;
    let manual = infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    let archived = manual
        .change_status(
            db.owner,
            db.case,
            typed.id,
            typed.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let ParticipantRevisionSnapshot::Typed(archived) = archived.revision else {
        panic!()
    };
    assert_eq!(archived.submission_revision, typed.revision);
    assert_eq!(archived.credential_origin, typed.credential_origin);
    assert_eq!(
        store(&db)
            .credential(db.owner, db.case, typed.id, archived.revision, db.at)
            .unwrap(),
        evidence
    );
    let subject = typed.values.subject();
    let other = ParticipantRoleValues::new(
        None,
        None,
        ParticipantProfile::Other(OtherParticipantProfile::new(
            ParticipantText::new("Other role").unwrap(),
            Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        )),
        typed.values.role().role_support().clone(),
    )
    .unwrap();
    let next = workflow
        .review_participant(
            "session",
            db.case,
            ParticipantProposalRequest {
                subject: SubjectDraftSelection::Keep(subject),
                participant: ParticipantDraftTarget::Existing {
                    id: typed.id,
                    expected_revision: archived.revision,
                },
                role: other,
                certificate: None,
            },
        )
        .unwrap();
    workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: reviewed(next),
                signature: None,
            },
        )
        .unwrap();
    let support =
        ParticipantEvidenceLocator::new(subject_support(&record), record.digest, "page 2").unwrap();
    let mut candidate = proposal(&record);
    candidate.subject = SubjectDraftSelection::Create(SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Another declaration").unwrap()),
        Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        support,
    ));
    candidate.certificate = Some(fx.leaf.clone());
    let review = workflow
        .review_participant("session", db.case, candidate)
        .unwrap();
    assert_eq!(review.candidates.len(), 1);
    assert_eq!(
        review.candidates[0].signals,
        vec![IdentityCandidateSignal::Certificate]
    );
    let second = ParticipantProposalRequest {
        subject: SubjectDraftSelection::Keep(subject),
        participant: ParticipantDraftTarget::Create,
        role: typed.values.role().clone(),
        certificate: Some(fx.leaf.clone()),
    };
    let mut second_request = reviewed(
        workflow
            .review_participant("session", db.case, second)
            .unwrap(),
    );
    second_request.certificate = Some(fx.leaf.clone());
    let second_draft = workflow
        .prepare_participant("session", db.case, second_request.clone())
        .unwrap()
        .declaration
        .unwrap();
    let signature = sign_statement(fx, &second_draft.bytes);
    workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: second_request,
                signature: Some(signature.clone()),
            },
        )
        .unwrap();
    let reader = store(&db);
    db.admin
        .batch_execute("ALTER TABLE participant_credential_evidence DISABLE TRIGGER USER")
        .unwrap();
    db.admin.execute("UPDATE participant_credential_evidence SET declaration=$2,statement_digest=sha256($2),signature=$3 WHERE participant_id=$1 AND revision=1",&[&typed.id.as_uuid(),&second_draft.bytes,&signature.as_bytes()]).unwrap();
    db.admin
        .batch_execute("ALTER TABLE participant_credential_evidence ENABLE TRIGGER USER")
        .unwrap();
    let before = snapshot(&mut db);
    let result = reader.credential(db.owner, db.case, typed.id, typed.revision, db.at);
    assert!(
        matches!(
            result,
            Err(ApplicationError::StoredParticipantInconsistent(_))
        ),
        "{result:?}"
    );
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute("ALTER TABLE participant_credential_evidence DISABLE TRIGGER USER")
        .unwrap();
    db.admin.execute(
        "UPDATE participant_credential_evidence SET declaration=$2,statement_digest=sha256($2),signature=$3,checked_at=$4 WHERE participant_id=$1 AND revision=1",
        &[&typed.id.as_uuid(), &evidence.declaration, &evidence.check.signature.as_bytes(), &(fx.at + 1)],
    ).unwrap();
    db.admin
        .batch_execute("ALTER TABLE participant_credential_evidence ENABLE TRIGGER USER")
        .unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        reader.credential(db.owner, db.case, typed.id, typed.revision, db.at),
        Err(ApplicationError::StoredParticipantInconsistent(_))
    ));
    assert_eq!(snapshot(&mut db), before);
}

fn subject_support(
    record: &application::documents::DocumentRecord,
) -> domain::crypto::DocumentVersionRef {
    domain::crypto::DocumentVersionRef {
        id: record.id,
        version: record.version,
    }
}

fn sign_statement(fx: &declaration_fixture::Fixture, bytes: &[u8]) -> Signature {
    let directory = tempfile::tempdir().unwrap();
    let statement = directory.path().join("statement.bin");
    std::fs::write(&statement, bytes).unwrap();
    Signature::from_bytes(declaration_fixture::openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        fx.ca
            .join("private/synthetic-declarant.key.pem")
            .to_str()
            .unwrap(),
        statement.to_str().unwrap(),
    ]))
    .unwrap()
}
