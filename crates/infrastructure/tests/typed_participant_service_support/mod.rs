#![allow(dead_code)]
pub use crate::case_administration_support::Fixture;
pub use crate::case_stage_database_support::{
    processor, upload, FixedClock, FormatCheck, TestIdentity,
};
use application::{identity::Principal, typed_participants::*};
use domain::{crypto::DocumentVersionRef, identity::Role};
use infrastructure::{
    certificates::InternalRsaDeclarationVerifier, PostgresTypedParticipantStore, RingSha256Hasher,
};
use std::sync::Arc;
pub fn store(db: &Fixture) -> Arc<PostgresTypedParticipantStore> {
    Arc::new(
        PostgresTypedParticipantStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, format: FormatCheck) -> TypedParticipantService {
    TypedParticipantService::new(
        Arc::new(TestIdentity(Principal {
            id: db.owner,
            email: "session@example.test".into(),
            role: Role::Owner,
        })),
        store(db),
        processor(),
        Arc::new(RingSha256Hasher),
        Arc::new(format),
        Arc::new(InternalRsaDeclarationVerifier),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn proposal(record: &application::documents::DocumentRecord) -> ParticipantProposalRequest {
    let support = ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
        "page 1",
    )
    .unwrap();
    ParticipantProposalRequest {
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
    }
}
pub fn reviewed(review: ParticipantProposalReview) -> ParticipantPreparationRequest {
    assert!(review.candidates.is_empty());
    ParticipantPreparationRequest {
        proposal: review.proposal,
        review: IdentityReviewSubmission {
            directory_stamp: review.directory_stamp,
            different: vec![],
            selection_reason: ParticipantReason::new("Identity and exact evidence reviewed")
                .unwrap(),
        },
        certificate: None,
    }
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    snapshot_client(&mut db.admin)
}
pub fn snapshot_client(client: &mut postgres::Client) -> serde_json::Value {
    client.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_subjects r),'subjects',(SELECT jsonb_agg(to_jsonb(r) ORDER BY subject_id,revision) FROM case_subject_revisions r),'participants',(SELECT jsonb_agg(to_jsonb(r) ORDER BY participant_id,revision) FROM case_participant_typed_revisions r),'subject_reviews',(SELECT jsonb_agg(to_jsonb(r) ORDER BY subject_id,revision) FROM subject_identity_reviews r),'reviews',(SELECT jsonb_agg(to_jsonb(r) ORDER BY participant_id,revision) FROM participant_identity_reviews r),'credentials',(SELECT jsonb_agg(to_jsonb(r) ORDER BY participant_id,revision) FROM participant_credential_evidence r),'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))",&[]).unwrap().get(0)
}
