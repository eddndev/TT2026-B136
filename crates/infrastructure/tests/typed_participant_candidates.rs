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

fn named(
    record: &application::documents::DocumentRecord,
    name: RepresentedName,
    locator: &str,
) -> ParticipantProposalRequest {
    let mut request = proposal(record);
    request.subject = SubjectDraftSelection::Create(SubjectValues::natural_person(
        name,
        Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        ParticipantEvidenceLocator::new(
            domain::crypto::DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
            locator,
        )
        .unwrap(),
    ));
    request
}

fn known(name: &str) -> RepresentedName {
    RepresentedName::Known(ParticipantText::new(name).unwrap())
}

fn submit(
    workflow: &TypedParticipantService,
    db: &Fixture,
    request: ParticipantPreparationRequest,
) {
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
}

#[test]
fn homonyms_require_exact_candidate_decisions_and_support_locators_do_not_merge_people() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "identity.pdf");
    let workflow = service(&db, FormatCheck(None));
    let first = workflow
        .review_participant("session", db.case, named(&record, known("Ana"), "page 1"))
        .unwrap();
    let first_subject = first.proposal.values().subject();
    submit(&workflow, &db, reviewed(first));
    let next = workflow
        .review_participant("session", db.case, named(&record, known("ANA"), "page 2"))
        .unwrap();
    assert_eq!(next.candidates.len(), 1);
    assert_eq!(
        next.candidates[0].signals,
        vec![IdentityCandidateSignal::Name]
    );
    let decision = IdentityDifferentDecision {
        candidate: next.candidates[0].reference.clone(),
        reason: ParticipantText::new("Distinct people on separate pages").unwrap(),
        support: next.proposal.values().role().role_support().clone(),
    };
    let mut request = ParticipantPreparationRequest {
        proposal: next.proposal,
        review: IdentityReviewSubmission {
            directory_stamp: next.directory_stamp,
            different: vec![],
            selection_reason: ParticipantReason::new("Compared both declared people").unwrap(),
        },
        certificate: None,
    };
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.prepare_participant("session", db.case, request.clone()),
        Err(ApplicationError::ParticipantIdentityReviewRequired)
    ));
    assert_eq!(snapshot(&mut db), before);
    let mut unrelated = decision.clone();
    unrelated.candidate = IdentityCandidateRef::Subject {
        id: CaseSubjectId::new(),
        revision: SubjectRevision::initial(),
    };
    request.review.different = vec![decision.clone(), unrelated];
    assert!(matches!(
        workflow.prepare_participant("session", db.case, request.clone()),
        Err(ApplicationError::ParticipantIdentityReviewRequired)
    ));
    assert_eq!(snapshot(&mut db), before);
    request.review.different = vec![decision];
    submit(&workflow, &db, request);
    let by_locator = workflow
        .review_participant(
            "session",
            db.case,
            named(&record, known("Distinct name"), "page 1"),
        )
        .unwrap();
    assert_eq!(by_locator.candidates.len(), 1);
    assert_eq!(
        by_locator.candidates[0].reference,
        IdentityCandidateRef::Subject {
            id: first_subject.id,
            revision: first_subject.revision,
        }
    );
    assert_eq!(
        by_locator.candidates[0].signals,
        vec![IdentityCandidateSignal::DocumentaryEvidence]
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_subjects", &[])
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}

#[test]
fn unidentified_labels_are_not_known_names_and_selected_subject_is_excluded() {
    let Some(db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "identity.pdf");
    let workflow = service(&db, FormatCheck(None));
    let label = || RepresentedName::Unidentified {
        label: ParticipantText::new("Unidentified person").unwrap(),
        reason: ParticipantReason::new("Identity not declared").unwrap(),
    };
    let first = workflow
        .review_participant("session", db.case, named(&record, label(), "page 1"))
        .unwrap();
    let subject = first.proposal.values().subject();
    submit(&workflow, &db, reviewed(first));
    for name in [label(), known("Unidentified person")] {
        let next = workflow
            .review_participant("session", db.case, named(&record, name, "page 2"))
            .unwrap();
        assert!(next.candidates.is_empty());
    }
    let mut selected = proposal(&record);
    selected.subject = SubjectDraftSelection::Keep(subject);
    let next = workflow
        .review_participant("session", db.case, selected)
        .unwrap();
    assert!(next.candidates.is_empty());
}

#[test]
fn candidate_scan_crosses_pages_and_capacity_applies_to_the_whole_directory() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "identity.pdf");
    let manual =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    for number in 1..=65 {
        manual
            .create(
                db.owner,
                db.case,
                ParticipantId::from_uuid(uuid::Uuid::from_u128(number)),
                application::participants::ParticipantValues::new(
                    if number == 65 {
                        "Ana"
                    } else {
                        "Different person"
                    },
                    "Declared role",
                    None,
                    None,
                    DirectoryStatus::Active,
                )
                .unwrap(),
                db.at,
            )
            .unwrap();
    }
    let workflow = service(&db, FormatCheck(None));
    let review = workflow
        .review_participant("session", db.case, proposal(&record))
        .unwrap();
    assert_eq!(review.candidates.len(), 1);
    let candidate = &review.candidates[0];
    assert_eq!(
        candidate.reference,
        IdentityCandidateRef::ManualParticipant {
            id: ParticipantId::from_uuid(uuid::Uuid::from_u128(65)),
            revision: ParticipantRevision::initial(),
        }
    );
    let mut selected = proposal(&record);
    selected.participant = ParticipantDraftTarget::Existing {
        id: ParticipantId::from_uuid(uuid::Uuid::from_u128(65)),
        expected_revision: ParticipantRevision::initial(),
    };
    let conversion = workflow
        .review_participant("session", db.case, selected)
        .unwrap();
    assert!(conversion.candidates.is_empty());
    submit(&workflow, &db, reviewed(conversion));
    for number in 66..=81 {
        manual
            .create(
                db.owner,
                db.case,
                ParticipantId::from_uuid(uuid::Uuid::from_u128(number)),
                application::participants::ParticipantValues::new(
                    "Ana",
                    "Declared role",
                    None,
                    None,
                    DirectoryStatus::Active,
                )
                .unwrap(),
                db.at,
            )
            .unwrap();
    }
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.review_participant("session", db.case, proposal(&record)),
        Err(ApplicationError::ParticipantCandidateLimit)
    ));
    assert_eq!(snapshot(&mut db), before);
}
