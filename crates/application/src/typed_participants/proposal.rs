use super::*;
use domain::crypto::{CredentialCertificate, Sha256Digest, Signature};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectDraftSelection {
    Create(SubjectValues),
    Keep(SubjectRevisionRef),
    Replace {
        current: SubjectRevisionRef,
        values: SubjectValues,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantDraftTarget {
    Create,
    Existing {
        id: ParticipantId,
        expected_revision: ParticipantRevision,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantProposalRequest {
    pub subject: SubjectDraftSelection,
    pub participant: ParticipantDraftTarget,
    pub role: ParticipantRoleValues,
    pub certificate: Option<Vec<u8>>, // only public cert, bounded before inspect
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectChange {
    Keep(SubjectRevisionRef),
    Append {
        id: CaseSubjectId,
        expected: SubjectExpectation,
        values: SubjectValues,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantProposal {
    // Validated private fields with same-name getters; service assigns new IDs.
    subject: SubjectChange,
    participant_id: ParticipantId,
    expected_participant: ParticipantExpectation,
    values: TypedParticipantValues,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantProposalReview {
    pub proposal: ParticipantProposal,
    pub directory_stamp: CaseDirectoryStamp,
    pub candidates: Vec<IdentityCandidate>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantPreparationRequest {
    pub proposal: ParticipantProposal,
    pub review: IdentityReviewSubmission,
    pub certificate: Option<Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantSigningDraft {
    pub proposal: ParticipantProposal,
    pub review: IdentityReviewSubmission,
    pub declaration: Option<ParticipantDeclaration>,
    pub submission_digest: Option<Sha256Digest>,
    pub submission_revision: ParticipantRevision,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantDeclaration {
    pub bytes: Vec<u8>,
    pub digest: Sha256Digest,
    pub certificate: CredentialCertificate,
    pub deployment_id: Uuid,
    pub trust_revision: CredentialTrustRevision,
    pub root_fingerprint: Sha256Digest,
    pub participant_values_digest: Sha256Digest,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantSubmission {
    pub prepared: ParticipantPreparationRequest,
    pub signature: Option<Signature>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectReplacementRequest {
    pub id: CaseSubjectId,
    pub expected_revision: SubjectRevision,
    pub values: SubjectValues,
    pub review: IdentityReviewSubmission,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectReplacementReview {
    pub id: CaseSubjectId,
    pub expected_revision: SubjectRevision,
    pub values: SubjectValues,
    pub directory_stamp: CaseDirectoryStamp,
    pub candidates: Vec<IdentityCandidate>,
}

impl ParticipantProposal {
    /// Reconstructs structure; the service recomputes hashes before preparation.
    pub fn new(
        subject: SubjectChange,
        participant_id: ParticipantId,
        expected_participant: ParticipantExpectation,
        values: TypedParticipantValues,
    ) -> Result<Self, ApplicationError> {
        if expected_participant == ParticipantExpectation::Absent
            && values.directory_status() != DirectoryStatus::Active
        {
            return Err(ApplicationError::InvalidInput(
                "new typed participant must be active".into(),
            ));
        }
        expected_participant
            .next()
            .ok_or(ApplicationError::ParticipantRevisionExhausted)?;
        let reference = values.subject();
        let coherent = match &subject {
            SubjectChange::Keep(current) => *current == reference,
            SubjectChange::Append { id, expected, .. } => {
                *id == reference.id
                    && expected
                        .next()
                        .ok_or(ApplicationError::SubjectRevisionExhausted)?
                        == reference.revision
            }
        };
        if !coherent {
            return Err(ApplicationError::InvalidInput(
                "proposed subject reference is inconsistent".into(),
            ));
        }
        Ok(Self {
            subject,
            participant_id,
            expected_participant,
            values,
        })
    }
    pub fn subject(&self) -> &SubjectChange {
        &self.subject
    }
    pub const fn participant_id(&self) -> ParticipantId {
        self.participant_id
    }
    pub const fn expected_participant(&self) -> ParticipantExpectation {
        self.expected_participant
    }
    pub fn values(&self) -> &TypedParticipantValues {
        &self.values
    }
}
impl SubjectChange {
    pub const fn id(&self) -> CaseSubjectId {
        match self {
            Self::Keep(v) => v.id,
            Self::Append { id, .. } => *id,
        }
    }
    pub const fn expectation(&self) -> SubjectExpectation {
        match self {
            Self::Keep(v) => SubjectExpectation::Revision(v.revision),
            Self::Append { expected, .. } => *expected,
        }
    }
}
