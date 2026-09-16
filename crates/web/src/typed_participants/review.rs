use super::values::{digest, Locator};
use application::{typed_participants as m, ApplicationError as Error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum CandidateRef {
    Subject { id: Uuid, revision: u32 },
    ManualParticipant { id: Uuid, revision: u32 },
}
impl CandidateRef {
    fn validate(self) -> Result<m::IdentityCandidateRef, Error> {
        Ok(match self {
            Self::Subject { id, revision } => m::IdentityCandidateRef::Subject {
                id: m::CaseSubjectId::from_uuid(id),
                revision: m::SubjectRevision::new(revision)?,
            },
            Self::ManualParticipant { id, revision } => {
                m::IdentityCandidateRef::ManualParticipant {
                    id: m::ParticipantId::from_uuid(id),
                    revision: m::ParticipantRevision::new(revision)?,
                }
            }
        })
    }
}
impl From<&m::IdentityCandidateRef> for CandidateRef {
    fn from(value: &m::IdentityCandidateRef) -> Self {
        match value {
            m::IdentityCandidateRef::Subject { id, revision } => Self::Subject {
                id: id.as_uuid(),
                revision: revision.get(),
            },
            m::IdentityCandidateRef::ManualParticipant { id, revision } => {
                Self::ManualParticipant {
                    id: id.as_uuid(),
                    revision: revision.get(),
                }
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Different {
    candidate: CandidateRef,
    reason: String,
    support: Locator,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Review {
    directory_stamp: String,
    selection_reason: String,
    different: Vec<Different>,
}
impl Review {
    pub fn validate(self) -> Result<m::IdentityReviewSubmission, Error> {
        if self.different.len() > 16 {
            return Err(Error::ParticipantCandidateLimit);
        }
        let result = m::IdentityReviewSubmission {
            directory_stamp: m::CaseDirectoryStamp(digest(&self.directory_stamp)?),
            selection_reason: m::ParticipantReason::new(&self.selection_reason)?,
            different: self
                .different
                .into_iter()
                .map(|d| {
                    Ok(m::IdentityDifferentDecision {
                        candidate: d.candidate.validate()?,
                        reason: m::ParticipantText::new(&d.reason)?,
                        support: d.support.validate()?,
                    })
                })
                .collect::<Result<_, Error>>()?,
        };
        result.canonical_bytes()?;
        Ok(result)
    }
}
impl From<&m::IdentityReviewSubmission> for Review {
    fn from(value: &m::IdentityReviewSubmission) -> Self {
        Self {
            directory_stamp: value.directory_stamp.0.to_hex(),
            selection_reason: value.selection_reason.as_str().into(),
            different: value
                .different
                .iter()
                .map(|d| Different {
                    candidate: (&d.candidate).into(),
                    reason: d.reason.as_str().into(),
                    support: (&d.support).into(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub(super) struct Candidate {
    reference: CandidateRef,
    display_name: String,
    kind: Option<&'static str>,
    signals: Vec<&'static str>,
}
impl From<m::IdentityCandidate> for Candidate {
    fn from(value: m::IdentityCandidate) -> Self {
        Self {
            reference: (&value.reference).into(),
            display_name: value.display_name,
            kind: value.kind.map(|v| v.as_str()),
            signals: value
                .signals
                .into_iter()
                .map(|s| match s {
                    m::IdentityCandidateSignal::Name => "name",
                    m::IdentityCandidateSignal::DeclaredIdentifier => "declared_identifier",
                    m::IdentityCandidateSignal::Certificate => "certificate",
                    m::IdentityCandidateSignal::DocumentaryEvidence => "documentary_evidence",
                })
                .collect(),
        }
    }
}
