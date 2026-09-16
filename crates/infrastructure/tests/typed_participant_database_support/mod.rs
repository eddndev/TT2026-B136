#![allow(dead_code)]
use crate::case_administration_support::Fixture;
use application::typed_participants::*;
use domain::{
    crypto::{DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    participants::ParticipantValues,
};
use infrastructure::RingSha256Hasher;
use postgres::GenericClient;
use time::format_description::well_known::Rfc3339;

pub struct Bundle {
    pub subject: CaseSubjectId,
    pub participant: ParticipantId,
    pub request: ParticipantPreparationRequest,
}
impl Bundle {
    pub fn new(db: &Fixture) -> Self {
        let subject = CaseSubjectId::new();
        let participant = ParticipantId::new();
        let support = ParticipantEvidenceLocator::new(
            DocumentVersionRef {
                id: DocumentId::new(),
                version: DocumentVersion::new(1).unwrap(),
            },
            Sha256Digest::from_bytes(&[17; 32]).unwrap(),
            "page 1",
        )
        .unwrap();
        let values = SubjectValues::natural_person(
            RepresentedName::Known(ParticipantText::new("Ana").unwrap()),
            Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
            support.clone(),
        );
        let bound = SubjectRevisionRef {
            id: subject,
            revision: SubjectRevision::new(1).unwrap(),
            values_digest: subject_digest(&RingSha256Hasher, &values),
        };
        let typed = TypedParticipantValues::new(
            bound,
            DirectoryStatus::Active,
            ParticipantRoleValues::new(
                None,
                None,
                ParticipantProfile::Defendant(DefendantProfile::new(Declared::Known(
                    CustodyState::AtLiberty,
                ))),
                support,
            )
            .unwrap(),
        );
        let proposal = ParticipantProposal::new(
            SubjectChange::Append {
                id: subject,
                expected: SubjectExpectation::Absent,
                values,
            },
            participant,
            ParticipantExpectation::Revision(ParticipantRevision::initial()),
            typed,
        )
        .unwrap();
        Self {
            subject,
            participant,
            request: ParticipantPreparationRequest {
                proposal,
                certificate: None,
                review: IdentityReviewSubmission {
                    directory_stamp: directory_stamp_seed(&RingSha256Hasher, db.case),
                    different: vec![],
                    selection_reason: ParticipantReason::new("Staff reviewed identity").unwrap(),
                },
            },
        }
    }
    pub fn seed_document(&self, tx: &mut impl GenericClient, db: &Fixture) {
        let reference = self
            .request
            .proposal
            .values()
            .role()
            .role_support()
            .reference();
        tx.execute(
            "INSERT INTO document_series(id,case_id,first_available_version) VALUES($1,$2,1)",
            &[&reference.id.as_uuid(), &db.case.as_uuid()],
        )
        .unwrap();
        tx.execute("INSERT INTO documents(id,version,case_id,name,digest,vault) VALUES($1,1,$2,'identity.pdf',$3,$4)",&[&reference.id.as_uuid(),&db.case.as_uuid(),&&[17u8;32][..],&vec![0u8]]).unwrap();
    }
    pub fn seed_manual(&self, tx: &mut impl GenericClient, db: &Fixture) {
        tx.execute(
            "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
            &[&self.participant.as_uuid(), &db.case.as_uuid()],
        )
        .unwrap();
        let at = db.at.format(&Rfc3339).unwrap();
        tx.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,organization,legal_status,directory_status,values_digest,changed_at,changed_by,changed_by_email) VALUES($1,1,'Ana','Declared role',NULL,NULL,'active',sha256(participant_values_bytes('Ana','Declared role',NULL,NULL,'active')),$2,$3,'owner@example.test')",&[&self.participant.as_uuid(),&at,&db.owner.as_uuid()]).unwrap();
    }
    pub fn refresh_stamp(&mut self, db: &Fixture) {
        let manual =
            ParticipantValues::new("Ana", "Declared role", None, None, DirectoryStatus::Active)
                .unwrap();
        self.request.review.directory_stamp = advance_directory_stamp(
            &RingSha256Hasher,
            directory_stamp_seed(&RingSha256Hasher, db.case),
            1,
            self.participant.as_uuid(),
            1,
            RingSha256Hasher.hash_bytes(&manual.canonical_bytes()),
        );
    }
    pub fn insert(&self, tx: &mut impl GenericClient, db: &Fixture) {
        let SubjectChange::Append { values, .. } = self.request.proposal.subject() else {
            panic!()
        };
        let canonical = values.canonical_bytes();
        let digest = subject_digest(&RingSha256Hasher, values);
        let support = values.identity_support();
        let at = db.at.format(&Rfc3339).unwrap();
        tx.execute(
            "INSERT INTO case_subjects(id,case_id) VALUES($1,$2)",
            &[&self.subject.as_uuid(), &db.case.as_uuid()],
        )
        .unwrap();
        tx.execute("INSERT INTO case_subject_revisions(subject_id,revision,values_canonical,values_digest,subject_kind,display_name,name_known,declared_identifier,identity_document_id,identity_document_version,identity_document_digest,identity_document_locator,changed_at,changed_by,changed_by_email) VALUES($1,1,$2,$3,'natural_person','Ana',TRUE,NULL,$4,1,$5,'page 1',$6,$7,'owner@example.test')",&[&self.subject.as_uuid(),&canonical,&digest.as_bytes().as_slice(),&support.reference().id.as_uuid(),&support.digest().as_bytes().as_slice(),&at,&db.owner.as_uuid()]).unwrap();
        let review = self.request.review.canonical_bytes().unwrap();
        tx.execute("INSERT INTO subject_identity_reviews(subject_id,revision,review_canonical,review_digest) VALUES($1,1,$2,sha256($2))",&[&self.subject.as_uuid(),&review]).unwrap();
        let submission =
            participant_submission_bytes(&RingSha256Hasher, db.case, &self.request, None).unwrap();
        let canonical = self.request.proposal.values().canonical_bytes();
        tx.execute("INSERT INTO case_participant_typed_revisions(participant_id,revision,values_canonical,values_digest,subject_id,subject_revision,role_kind,organization,directory_status,changed_at,changed_by,changed_by_email,submission_digest,submission_revision) VALUES($1,2,$2,sha256($2),$3,1,'defendant',NULL,'active',$4,$5,'owner@example.test',sha256($6),2)",&[&self.participant.as_uuid(),&canonical,&self.subject.as_uuid(),&at,&db.owner.as_uuid(),&submission]).unwrap();
        tx.execute("INSERT INTO participant_identity_reviews(participant_id,revision,review_canonical,review_digest,submission_canonical,submission_digest) VALUES($1,2,$2,sha256($2),$3,sha256($3))",&[&self.participant.as_uuid(),&review,&submission]).unwrap();
    }
}
