use super::{
    profile::Profile,
    review::Review,
    values::{Locator, SubjectRef, SubjectValues},
};
use application::{typed_participants as m, ApplicationError as Error};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::crypto::{CredentialFailure, Signature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Role {
    organization: Option<String>,
    legal_status: Option<String>,
    profile: Profile,
    role_support: Locator,
}
impl Role {
    fn validate(self) -> Result<m::ParticipantRoleValues, Error> {
        Ok(m::ParticipantRoleValues::new(
            self.organization.as_deref(),
            self.legal_status.as_deref(),
            self.profile.validate()?,
            self.role_support.validate()?,
        )?)
    }
}
impl From<&m::ParticipantRoleValues> for Role {
    fn from(v: &m::ParticipantRoleValues) -> Self {
        Self {
            organization: v.organization().map(str::to_owned),
            legal_status: v.legal_status().map(str::to_owned),
            profile: v.profile().into(),
            role_support: v.role_support().into(),
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Values {
    subject: SubjectRef,
    directory_status: m::DirectoryStatus,
    role: Role,
}
impl Values {
    fn validate(self) -> Result<m::TypedParticipantValues, Error> {
        Ok(m::TypedParticipantValues::new(
            self.subject.validate()?,
            self.directory_status,
            self.role.validate()?,
        ))
    }
}
impl From<&m::TypedParticipantValues> for Values {
    fn from(v: &m::TypedParticipantValues) -> Self {
        Self {
            subject: v.subject().into(),
            directory_status: v.directory_status(),
            role: v.role().into(),
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum SubjectSelection {
    Create {
        values: SubjectValues,
    },
    Keep {
        reference: SubjectRef,
    },
    Replace {
        current: SubjectRef,
        values: SubjectValues,
    },
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Target {
    Create,
    Existing { id: Uuid, expected_revision: u32 },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewRequest {
    subject: SubjectSelection,
    participant: Target,
    role: Role,
    certificate_base64: Option<String>,
}
impl ReviewRequest {
    pub fn validate(self) -> Result<m::ParticipantProposalRequest, Error> {
        Ok(m::ParticipantProposalRequest {
            subject: match self.subject {
                SubjectSelection::Create { values } => {
                    m::SubjectDraftSelection::Create(values.validate()?)
                }
                SubjectSelection::Keep { reference } => {
                    m::SubjectDraftSelection::Keep(reference.validate()?)
                }
                SubjectSelection::Replace { current, values } => {
                    m::SubjectDraftSelection::Replace {
                        current: current.validate()?,
                        values: values.validate()?,
                    }
                }
            },
            participant: match self.participant {
                Target::Create => m::ParticipantDraftTarget::Create,
                Target::Existing {
                    id,
                    expected_revision,
                } => m::ParticipantDraftTarget::Existing {
                    id: m::ParticipantId::from_uuid(id),
                    expected_revision: m::ParticipantRevision::new(expected_revision)?,
                },
            },
            role: self.role.validate()?,
            certificate: certificate(self.certificate_base64)?,
        })
    }
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum SubjectChange {
    Keep {
        reference: SubjectRef,
    },
    Append {
        id: Uuid,
        expected_revision: u32,
        values: SubjectValues,
    },
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proposal {
    subject: SubjectChange,
    participant_id: Uuid,
    expected_participant_revision: u32,
    values: Values,
}
impl Proposal {
    fn validate(self) -> Result<m::ParticipantProposal, Error> {
        let subject = match self.subject {
            SubjectChange::Keep { reference } => m::SubjectChange::Keep(reference.validate()?),
            SubjectChange::Append {
                id,
                expected_revision,
                values,
            } => m::SubjectChange::Append {
                id: m::CaseSubjectId::from_uuid(id),
                expected: match expected_revision {
                    0 => m::SubjectExpectation::Absent,
                    n => m::SubjectExpectation::Revision(m::SubjectRevision::new(n)?),
                },
                values: values.validate()?,
            },
        };
        let expected = match self.expected_participant_revision {
            0 => m::ParticipantExpectation::Absent,
            n => m::ParticipantExpectation::Revision(m::ParticipantRevision::new(n)?),
        };
        m::ParticipantProposal::new(
            subject,
            m::ParticipantId::from_uuid(self.participant_id),
            expected,
            self.values.validate()?,
        )
    }
}
impl From<&m::ParticipantProposal> for Proposal {
    fn from(v: &m::ParticipantProposal) -> Self {
        Self {
            subject: match v.subject() {
                m::SubjectChange::Keep(reference) => SubjectChange::Keep {
                    reference: (*reference).into(),
                },
                m::SubjectChange::Append {
                    id,
                    expected,
                    values,
                } => SubjectChange::Append {
                    id: id.as_uuid(),
                    expected_revision: expected.get(),
                    values: values.into(),
                },
            },
            participant_id: v.participant_id().as_uuid(),
            expected_participant_revision: v.expected_participant().get(),
            values: v.values().into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preparation {
    proposal: Proposal,
    review: Review,
    certificate_base64: Option<String>,
}
impl Preparation {
    pub fn validate(self) -> Result<m::ParticipantPreparationRequest, Error> {
        Ok(m::ParticipantPreparationRequest {
            proposal: self.proposal.validate()?,
            review: self.review.validate()?,
            certificate: certificate(self.certificate_base64)?,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    prepared: Preparation,
    signature_base64: Option<String>,
}
impl Submission {
    pub fn validate(self) -> Result<m::ParticipantSubmission, Error> {
        let signature = self
            .signature_base64
            .map(|value| -> Result<Signature, Error> {
                let bytes = decode(&value, 384)?;
                if bytes.len() != 384 {
                    return Err(CredentialFailure::InvalidSignature.into());
                }
                Ok(Signature::from_bytes(bytes)?)
            })
            .transpose()?;
        Ok(m::ParticipantSubmission {
            prepared: self.prepared.validate()?,
            signature,
        })
    }
}
fn certificate(value: Option<String>) -> Result<Option<Vec<u8>>, Error> {
    value.map(|v| decode(&v, 16 * 1024)).transpose()
}
fn decode(value: &str, limit: usize) -> Result<Vec<u8>, Error> {
    if value.len() > limit.div_ceil(3) * 4 {
        return Err(CredentialFailure::LimitExceeded.into());
    }
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| Error::InvalidInput("invalid padded standard base64".into()))?;
    if bytes.is_empty() {
        return Err(Error::InvalidInput("public material is empty".into()));
    }
    if bytes.len() > limit {
        return Err(CredentialFailure::LimitExceeded.into());
    }
    Ok(bytes)
}
