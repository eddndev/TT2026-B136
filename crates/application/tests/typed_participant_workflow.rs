#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod typed_participant_support;
use application::{typed_participants::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use typed_participant_support::*;

#[test]
fn unsigned_create_validates_one_batch_and_returns_commit_without_post_get() {
    for role in [Role::Owner, Role::Litigator] {
        let (identity, actor) = identity(role, 2);
        let case_id = CaseId::new();
        let record = crypto::processor()
            .prepare("identity.pdf", b"identity")
            .unwrap();
        let (request, subject) = request(&record);
        let prepared = preparation(&request, subject.clone(), vec![record]);
        let expected = detail(case_id, actor.id, &request, subject);
        let result = expected.clone();
        let mut store = MockStore::new();
        store
            .expect_prepare_participant()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prepared));
        store
            .expect_commit_participant()
            .times(1)
            .return_once(move |_, _, p| {
                assert_eq!(p.records().len(), 1);
                assert!(p.check().is_none());
                Ok(result)
            });
        let service = service(store, identity, false);
        assert_eq!(
            service
                .submit_participant(
                    "session",
                    case_id,
                    ParticipantSubmission {
                        prepared: request,
                        signature: None
                    }
                )
                .unwrap(),
            expected
        );
    }
}

#[test]
fn read_only_and_client_roles_cannot_prepare_or_commit_mutations() {
    for role in [Role::Paralegal, Role::Client] {
        let (identity, _) = identity(role, 1);
        let record = crypto::processor()
            .prepare("identity.pdf", b"identity")
            .unwrap();
        let (request, _) = request(&record);
        let service = service(MockStore::new(), identity, false);
        assert!(matches!(
            service.submit_participant(
                "session",
                CaseId::new(),
                ParticipantSubmission {
                    prepared: request,
                    signature: None
                }
            ),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn rejected_parser_or_changed_subject_has_no_commit() {
    for wrong_subject in [false, true] {
        let (identity, _) = identity(Role::Owner, 1);
        let record = crypto::processor()
            .prepare("identity.pdf", b"identity")
            .unwrap();
        let (request, mut subject) = request(&record);
        if wrong_subject {
            subject = SubjectValues::natural_person(
                RepresentedName::Known(ParticipantText::new("Another").unwrap()),
                Declared::Unknown(ParticipantReason::new("Awaiting record").unwrap()),
                subject.identity_support().clone(),
            );
        }
        let prepared = preparation(&request, subject, vec![record]);
        let mut store = MockStore::new();
        store
            .expect_prepare_participant()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prepared));
        let service = service(store, identity, true);
        assert!(service
            .submit_participant(
                "session",
                CaseId::new(),
                ParticipantSubmission {
                    prepared: request,
                    signature: None
                }
            )
            .is_err());
    }
}

#[test]
fn unsigned_draft_exposes_exact_submission_digest_before_commit() {
    let (identity, _) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let record = crypto::processor()
        .prepare("identity.pdf", b"identity")
        .unwrap();
    let (request, subject) = request(&record);
    let expected = participant_submission_digest(&Hasher, case_id, &request, None).unwrap();
    let prepared = preparation(&request, subject, vec![record]);
    let mut store = MockStore::new();
    store
        .expect_prepare_participant()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prepared));
    let draft = service(store, identity, false)
        .prepare_participant("session", case_id, request)
        .unwrap();
    assert_eq!(draft.submission_digest, Some(expected));
    assert_eq!(draft.submission_revision, ParticipantRevision::initial());
    assert!(draft.declaration.is_none());
}
