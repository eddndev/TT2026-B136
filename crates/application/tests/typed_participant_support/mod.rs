pub use super::case_support::{identity, instant, CountingClock, MockIdentity};
use application::documents::{DocumentFormatBatch, DocumentRecord, StageDocumentFormat};
use application::{typed_participants::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::*, identity::UserId, DomainError};
use mockall::mock;
use std::{io::Read, sync::Arc};
mock! { pub Store {} impl TypedParticipantStore for Store {

   fn review_participant(
       &self,
       actor: UserId,
       case_id: CaseId,
       request: ParticipantProposalRequest,
       proposed_subject: CaseSubjectId,
       proposed_participant: ParticipantId,
       certificate: Option<CredentialCertificate>,
       at: OffsetDateTime,
   ) -> Result<ParticipantProposalReview, ApplicationError>;
   fn prepare_participant(
       &self,
       actor: UserId,
       case_id: CaseId,
       request: &ParticipantPreparationRequest,
       limits: &StageSupportReadLimits,
   ) -> Result<TypedParticipantPreparation, ApplicationError>;
   fn commit_participant(
       &self,
       actor: UserId,
       case_id: CaseId,
       prepared: PreparedTypedParticipantChange,
   ) -> Result<ParticipantDetail, ApplicationError>;
   fn review_subject(
       &self,
       actor: UserId,
       case_id: CaseId,
       id: CaseSubjectId,
       expected: SubjectRevision,
       values: SubjectValues,
       at: OffsetDateTime,
   ) -> Result<SubjectReplacementReview, ApplicationError>;
   fn prepare_subject(
       &self,
       actor: UserId,
       case_id: CaseId,
       request: &SubjectReplacementRequest,
       limits: &StageSupportReadLimits,
   ) -> Result<SubjectPreparation, ApplicationError>;
   fn commit_subject(
       &self,
       actor: UserId,
       case_id: CaseId,
       prepared: PreparedSubjectChange,
   ) -> Result<SubjectSnapshot, ApplicationError>;
   // Read signatures mirror workflow, token replaced by actor:UserId,
   // with final at:OffsetDateTime argument; all commit read audit before return.
   fn list_subjects(
       &self,
       actor: UserId,
       case_id: CaseId,
       query: SubjectQuery,
       at: OffsetDateTime,
   ) -> Result<SubjectPage, ApplicationError>;
   fn get_subject(
       &self,
       actor: UserId,
       case_id: CaseId,
       id: CaseSubjectId,
       at: OffsetDateTime,
   ) -> Result<SubjectSnapshot, ApplicationError>;
   fn get_subject_revision(
       &self,
       actor: UserId,
       case_id: CaseId,
       id: CaseSubjectId,
       revision: SubjectRevision,
       at: OffsetDateTime,
   ) -> Result<SubjectSnapshot, ApplicationError>;
   fn subject_history(
       &self,
       actor: UserId,
       case_id: CaseId,
       id: CaseSubjectId,
       query: SubjectHistoryQuery,
       at: OffsetDateTime,
   ) -> Result<SubjectHistoryPage, ApplicationError>;
   fn credential(
       &self,
       actor: UserId,
       case_id: CaseId,
       id: ParticipantId,
       revision: ParticipantRevision,
       at: OffsetDateTime,
   ) -> Result<ParticipantCredentialEvidence, ApplicationError>;
} }
mock! { pub Credentials {} impl InternalDeclarationVerifier for Credentials {
    fn inspect_certificate(&self,certificate:&[u8])->Result<CredentialCertificate,CredentialFailure>;
    fn inspect_trust(&self,root:&[u8],crl:&[u8],at:i64)->Result<CredentialTrustInspection,CredentialFailure>;
    fn verify(&self,digest:&Sha256Digest,certificate:&[u8],signature:&Signature,root:&[u8],crl:&[u8],at:i64)->Result<CredentialCheck,CredentialFailure>;
} }
pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut result = [0u8; 32];
        for (i, b) in data.iter().enumerate() {
            result[i % 32] = result[i % 32].wrapping_add(*b).wrapping_add(i as u8);
        }
        Sha256Digest::from_array(result)
    }
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut value = Vec::new();
        reader.read_to_end(&mut value).unwrap();
        Ok(self.hash_bytes(&value))
    }
}
pub struct Validator {
    pub fail: bool,
}
impl DocumentFormatBatchValidator for Validator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        assert_eq!(batch.inputs().len(), 1);
        if self.fail {
            return Err(ApplicationError::StageSupportFormatRejected);
        }
        Ok(vec![StageDocumentFormat::Pdf])
    }
}
pub fn request(record: &DocumentRecord) -> (ParticipantPreparationRequest, SubjectValues) {
    let support = ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
        "page 1",
    )
    .unwrap();
    let subject_values = SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Name").unwrap()),
        Declared::Unknown(ParticipantReason::new("Awaiting record").unwrap()),
        support.clone(),
    );
    let reference = SubjectRevisionRef {
        id: CaseSubjectId::new(),
        revision: SubjectRevision::initial(),
        values_digest: subject_digest(&Hasher, &subject_values),
    };
    let role = ParticipantRoleValues::new(
        None,
        None,
        ParticipantProfile::Defendant(DefendantProfile::new(Declared::Known(
            CustodyState::AtLiberty,
        ))),
        support,
    )
    .unwrap();
    let values = TypedParticipantValues::new(reference, DirectoryStatus::Active, role);
    let proposal = ParticipantProposal::new(
        SubjectChange::Append {
            id: reference.id,
            expected: SubjectExpectation::Absent,
            values: subject_values.clone(),
        },
        ParticipantId::new(),
        ParticipantExpectation::Absent,
        values,
    )
    .unwrap();
    (
        ParticipantPreparationRequest {
            proposal,
            review: IdentityReviewSubmission {
                directory_stamp: CaseDirectoryStamp(Sha256Digest::from_array([2; 32])),
                different: vec![],
                selection_reason: ParticipantReason::new("Examined source").unwrap(),
            },
            certificate: None,
        },
        subject_values,
    )
}
pub fn preparation(
    request: &ParticipantPreparationRequest,
    subject_values: SubjectValues,
    records: Vec<DocumentRecord>,
) -> TypedParticipantPreparation {
    TypedParticipantPreparation {
        directory_stamp: request.review.directory_stamp.clone(),
        subject_values,
        records,
        trust: None,
    }
}
pub fn service(store: MockStore, identity: MockIdentity, fail: bool) -> TypedParticipantService {
    TypedParticipantService::new(
        Arc::new(identity),
        Arc::new(store),
        Arc::new(super::crypto::processor()),
        Arc::new(Hasher),
        Arc::new(Validator { fail }),
        Arc::new(MockCredentials::new()),
        Arc::new(CountingClock::default()),
    )
}
pub fn detail(
    case_id: CaseId,
    actor: UserId,
    request: &ParticipantPreparationRequest,
    subject_values: SubjectValues,
) -> ParticipantDetail {
    let subject = request.proposal.values().subject();
    ParticipantDetail {
        revision: ParticipantRevisionSnapshot::Typed(Box::new(TypedParticipantSnapshot {
            case_id,
            id: request.proposal.participant_id(),
            revision: ParticipantRevision::initial(),
            values: request.proposal.values().clone(),
            values_digest: typed_participant_digest(&Hasher, request.proposal.values()),
            changed_at: instant(),
            changed_by: ParticipantActorSnapshot {
                id: actor,
                email: "committed@example.com".into(),
            },
            credential_origin: None,
            submission_digest: participant_submission_digest(&Hasher, case_id, request, None)
                .unwrap(),
            submission_revision: ParticipantRevision::initial(),
        })),
        bound_subject: Some(SubjectSnapshot {
            case_id,
            id: subject.id,
            revision: subject.revision,
            values: subject_values,
            values_digest: subject.values_digest,
            changed_at: instant(),
            changed_by: ParticipantActorSnapshot {
                id: actor,
                email: "committed@example.com".into(),
            },
        }),
    }
}
