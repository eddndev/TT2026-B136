mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod typed_participant_service_support;
use application::{typed_participants::*, ApplicationError};
use typed_participant_service_support::*;
#[test]
fn unsigned_identity_and_role_are_atomic_and_keep_exact_captured_subject_history() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "identity.pdf");
    let workflow = service(&db, FormatCheck(None));
    let request = reviewed(
        workflow
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    let audit_before: i64 = db
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    let draft = workflow
        .prepare_participant("session", db.case, request.clone())
        .unwrap();
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get::<_, i64>(0),
        audit_before + 1
    );
    let committed = workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request.clone(),
                signature: None,
            },
        )
        .unwrap();
    let ParticipantRevisionSnapshot::Typed(ref typed) = committed.revision else {
        panic!()
    };
    assert_eq!(typed.submission_digest, draft.submission_digest.unwrap());
    assert_eq!(typed.revision.get(), 1);
    assert_eq!(typed.changed_by.email, "owner@example.test");
    let original = committed.bound_subject.clone().unwrap();
    assert_eq!(
        store(&db)
            .get_subject(db.owner, db.case, original.id, db.at)
            .unwrap(),
        original
    );
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None
            }
        ),
        Err(ApplicationError::ParticipantRevisionConflict)
    ));
    assert_eq!(snapshot(&mut db), before);
    let support = original.values.identity_support().clone();
    let new_values = SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Ana Updated").unwrap()),
        Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        support,
    );
    let reviewed = workflow
        .review_subject(
            "session",
            db.case,
            original.id,
            original.revision,
            new_values.clone(),
        )
        .unwrap();
    assert!(reviewed.candidates.is_empty());
    let updated = workflow
        .replace_subject(
            "session",
            db.case,
            SubjectReplacementRequest {
                id: original.id,
                expected_revision: original.revision,
                values: new_values,
                review: IdentityReviewSubmission {
                    directory_stamp: reviewed.directory_stamp,
                    different: vec![],
                    selection_reason: ParticipantReason::new("Updated declaration reviewed")
                        .unwrap(),
                },
            },
        )
        .unwrap();
    assert_eq!(updated.revision.get(), 2);
    assert_eq!(
        store(&db)
            .get_subject_revision(db.owner, db.case, original.id, original.revision, db.at)
            .unwrap(),
        original
    );
    assert_eq!(
        store(&db)
            .subject_history(
                db.owner,
                db.case,
                original.id,
                SubjectHistoryQuery::new(1, None).unwrap(),
                db.at
            )
            .unwrap()
            .revisions,
        vec![updated]
    );
}
