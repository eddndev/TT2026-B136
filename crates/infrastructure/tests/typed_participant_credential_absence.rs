mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod typed_participant_service_support;

use application::{participants::ParticipantStore, typed_participants::*, ApplicationError};
use infrastructure::{PostgresParticipantStore, RingSha256Hasher};
use std::sync::Arc;
use typed_participant_service_support::*;

#[test]
fn existing_manual_or_unsigned_revisions_report_absent_credential_without_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let manual = PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher))
        .unwrap()
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            application::participants::ParticipantValues::new(
                "Manual person",
                "Declared role",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let reader = store(&db);
    let other_case = domain::cases::CaseId::new();
    db.admin.execute(
        "INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Other baseline','OTHER',$2,NULL)",
        &[&other_case.as_uuid(), &db.owner.as_uuid()],
    ).unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        reader.credential(db.owner, db.case, manual.id, manual.revision, db.at),
        Err(ApplicationError::ParticipantCredentialNotFound)
    ));
    assert!(matches!(
        reader.credential(
            db.owner,
            db.case,
            manual.id,
            ParticipantRevision::new(2).unwrap(),
            db.at,
        ),
        Err(ApplicationError::ParticipantNotFound)
    ));
    assert!(matches!(
        reader.credential(db.owner, other_case, manual.id, manual.revision, db.at,),
        Err(ApplicationError::ParticipantNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
    let record = upload(&db, db.case, "identity.pdf");
    let workflow = service(&db, FormatCheck(None));
    let request = reviewed(
        workflow
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    let typed = workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        reader.credential(
            db.owner,
            db.case,
            typed.id(),
            typed.revision_number(),
            db.at
        ),
        Err(ApplicationError::ParticipantCredentialNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
}
