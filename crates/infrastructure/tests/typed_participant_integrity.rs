mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod typed_participant_service_support;
use application::{
    participants::{ParticipantStore, ParticipantValues},
    typed_participants::*,
    ApplicationError,
};
use std::sync::Arc;
use typed_participant_service_support::*;
#[test]
fn review_rejects_corrupted_head_provenance_in_either_revision_family() {
    for typed in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let record = upload(&db, db.case, "support.pdf");
        let workflow = service(&db, FormatCheck(None));
        let (id, revision) = if typed {
            let request = reviewed(
                workflow
                    .review_participant("session", db.case, proposal(&record))
                    .unwrap(),
            );
            let value = workflow
                .submit_participant(
                    "session",
                    db.case,
                    ParticipantSubmission {
                        prepared: request,
                        signature: None,
                    },
                )
                .unwrap();
            (value.id(), value.revision_number())
        } else {
            let manual = infrastructure::PostgresParticipantStore::open(
                &db.runtime_url,
                Arc::new(infrastructure::RingSha256Hasher),
            )
            .unwrap();
            let value = manual
                .create(
                    db.owner,
                    db.case,
                    ParticipantId::new(),
                    ParticipantValues::new("Ana", "Role", None, None, DirectoryStatus::Active)
                        .unwrap(),
                    db.at,
                )
                .unwrap();
            (value.id, value.revision)
        };
        let table = if typed {
            "case_participant_typed_revisions"
        } else {
            "case_participant_revisions"
        };
        db.admin.batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER USER; UPDATE {table} SET changed_by_email=E'bad\\n'; ALTER TABLE {table} ENABLE TRIGGER USER")).unwrap();
        let mut request = proposal(&record);
        request.participant = ParticipantDraftTarget::Existing {
            id,
            expected_revision: revision,
        };
        let before = snapshot(&mut db);
        let result = workflow.review_participant("session", db.case, request);
        assert!(
            matches!(
                result,
                Err(ApplicationError::StoredParticipantInconsistent(_))
            ),
            "{result:?}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}
#[test]
fn subject_index_rejects_corrupted_projection_and_provenance_after_open() {
    for field in ["display_name", "changed_by_email"] {
        let Some(mut db) = Fixture::new() else { return };
        let record = upload(&db, db.case, "support.pdf");
        let workflow = service(&db, FormatCheck(None));
        let request = reviewed(
            workflow
                .review_participant("session", db.case, proposal(&record))
                .unwrap(),
        );
        workflow
            .submit_participant(
                "session",
                db.case,
                ParticipantSubmission {
                    prepared: request,
                    signature: None,
                },
            )
            .unwrap();
        db.admin.batch_execute(&format!("ALTER TABLE case_subject_revisions DISABLE TRIGGER USER; ALTER TABLE case_subject_revisions DROP CONSTRAINT subject_projection; UPDATE case_subject_revisions SET {field}=E'bad\\n'; ALTER TABLE case_subject_revisions ENABLE TRIGGER USER")).unwrap();
        let before = snapshot(&mut db);
        let result = workflow.list_subjects(
            "session",
            db.case,
            SubjectQuery::new(10, None, None, None).unwrap(),
        );
        assert!(
            matches!(
                result,
                Err(ApplicationError::StoredParticipantInconsistent(_))
            ),
            "{result:?}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn a_corrupt_unselected_directory_head_cannot_be_used_as_an_identity_review_base() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let workflow = service(&db, FormatCheck(None));
    let request = reviewed(
        workflow
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        )
        .unwrap();
    db.admin.batch_execute("ALTER TABLE case_subject_revisions DISABLE TRIGGER USER;UPDATE case_subject_revisions SET changed_by_email=E'bad\\n';ALTER TABLE case_subject_revisions ENABLE TRIGGER USER").unwrap();
    let before = snapshot(&mut db);
    let result = workflow.review_participant("session", db.case, proposal(&record));
    assert!(
        matches!(
            result,
            Err(ApplicationError::StoredParticipantInconsistent(_))
        ),
        "{result:?}"
    );
    assert_eq!(snapshot(&mut db), before);
}
