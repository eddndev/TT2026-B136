mod case_administration_support;
use application::typed_participants::*;
use case_administration_support::Fixture;
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
};
use std::sync::Arc;
struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1735689600).unwrap()
    }
}
#[test]
fn typed_review_assigns_ids_without_rows_and_audits_staff_reads() {
    let Some(mut db) = Fixture::new() else { return };
    let store = infrastructure::PostgresTypedParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
        Arc::new(FixedClock),
    )
    .unwrap();
    let support = ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: DocumentId::new(),
            version: DocumentVersion::new(1).unwrap(),
        },
        Sha256Digest::from_array([7; 32]),
        "page 1",
    )
    .unwrap();
    let request = ParticipantProposalRequest {
        subject: SubjectDraftSelection::Create(SubjectValues::natural_person(
            RepresentedName::Known(ParticipantText::new("Ana").unwrap()),
            Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
            support.clone(),
        )),
        participant: ParticipantDraftTarget::Create,
        role: ParticipantRoleValues::new(
            None,
            None,
            ParticipantProfile::Defendant(DefendantProfile::new(Declared::Known(
                CustodyState::AtLiberty,
            ))),
            support,
        )
        .unwrap(),
        certificate: None,
    };
    let review = store
        .review_participant(
            db.owner,
            db.case,
            request,
            CaseSubjectId::new(),
            ParticipantId::new(),
            None,
            db.at,
        )
        .unwrap();
    assert!(review.candidates.is_empty());
    assert_eq!(
        review.proposal.expected_participant(),
        ParticipantExpectation::Absent
    );
    assert_eq!(
        store
            .list_subjects(
                db.owner,
                db.case,
                SubjectQuery::new(10, None, None, None).unwrap(),
                db.at
            )
            .unwrap()
            .subjects
            .len(),
        0
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}
