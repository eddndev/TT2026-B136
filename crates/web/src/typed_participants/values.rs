use application::{typed_participants as model, ApplicationError as Error};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Locator {
    document_id: Uuid,
    version: u32,
    digest: String,
    locator: String,
}
impl Locator {
    pub fn validate(self) -> Result<model::ParticipantEvidenceLocator, Error> {
        Ok(model::ParticipantEvidenceLocator::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(self.document_id),
                version: DocumentVersion::new(self.version)?,
            },
            digest(&self.digest)?,
            &self.locator,
        )?)
    }
}
impl From<&model::ParticipantEvidenceLocator> for Locator {
    fn from(value: &model::ParticipantEvidenceLocator) -> Self {
        Self {
            document_id: value.reference().id.as_uuid(),
            version: value.reference().version.get(),
            digest: value.digest().to_hex(),
            locator: value.locator().into(),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Declared<T> {
    Known { value: T },
    Unknown { reason: String },
}
impl<T> Declared<T> {
    pub fn validate<U>(
        self,
        convert: impl FnOnce(T) -> Result<U, Error>,
    ) -> Result<model::Declared<U>, Error> {
        Ok(match self {
            Self::Known { value } => model::Declared::Known(convert(value)?),
            Self::Unknown { reason } => {
                model::Declared::Unknown(model::ParticipantReason::new(&reason)?)
            }
        })
    }
    pub fn from_value<U>(value: &model::Declared<U>, convert: impl FnOnce(&U) -> T) -> Self {
        match value {
            model::Declared::Known(value) => Self::Known {
                value: convert(value),
            },
            model::Declared::Unknown(reason) => Self::Unknown {
                reason: reason.as_str().into(),
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Name {
    Known { value: String },
    Unidentified { label: String, reason: String },
}
impl Name {
    fn validate(self) -> Result<model::RepresentedName, Error> {
        Ok(match self {
            Self::Known { value } => model::RepresentedName::Known(text(value)?),
            Self::Unidentified { label, reason } => model::RepresentedName::Unidentified {
                label: text(label)?,
                reason: model::ParticipantReason::new(&reason)?,
            },
        })
    }
}
impl From<&model::RepresentedName> for Name {
    fn from(value: &model::RepresentedName) -> Self {
        match value {
            model::RepresentedName::Known(value) => Self::Known {
                value: value.as_str().into(),
            },
            model::RepresentedName::Unidentified { label, reason } => Self::Unidentified {
                label: label.as_str().into(),
                reason: reason.as_str().into(),
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum SubjectValues {
    NaturalPerson {
        name: Name,
        curp: Declared<String>,
        identity_support: Locator,
    },
    InstitutionalBody {
        name: String,
        institutional_identifier: Declared<String>,
        identity_support: Locator,
    },
}
impl SubjectValues {
    pub fn validate(self) -> Result<model::SubjectValues, Error> {
        Ok(match self {
            Self::NaturalPerson {
                name,
                curp,
                identity_support,
            } => model::SubjectValues::natural_person(
                name.validate()?,
                curp.validate(|v| Ok(model::Curp::new(&v)?))?,
                identity_support.validate()?,
            ),
            Self::InstitutionalBody {
                name,
                institutional_identifier,
                identity_support,
            } => model::SubjectValues::institutional_body(
                text(name)?,
                institutional_identifier.validate(text)?,
                identity_support.validate()?,
            ),
        })
    }
}
impl From<&model::SubjectValues> for SubjectValues {
    fn from(value: &model::SubjectValues) -> Self {
        match value {
            model::SubjectValues::NaturalPerson {
                name,
                curp,
                identity_support,
            } => Self::NaturalPerson {
                name: name.into(),
                curp: Declared::from_value(curp, |v| v.as_str().into()),
                identity_support: identity_support.into(),
            },
            model::SubjectValues::InstitutionalBody {
                name,
                institutional_identifier,
                identity_support,
            } => Self::InstitutionalBody {
                name: name.as_str().into(),
                institutional_identifier: Declared::from_value(institutional_identifier, |v| {
                    v.as_str().into()
                }),
                identity_support: identity_support.into(),
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct License {
    number: String,
    issuer: String,
}
impl License {
    pub fn validate(self) -> Result<model::ProfessionalLicense, Error> {
        Ok(model::ProfessionalLicense::new(&self.number, &self.issuer)?)
    }
}
impl From<&model::ProfessionalLicense> for License {
    fn from(value: &model::ProfessionalLicense) -> Self {
        Self {
            number: value.number().into(),
            issuer: value.issuer().into(),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SubjectRef {
    id: Uuid,
    revision: u32,
    values_digest: String,
}
impl SubjectRef {
    pub fn validate(self) -> Result<model::SubjectRevisionRef, Error> {
        Ok(model::SubjectRevisionRef {
            id: model::CaseSubjectId::from_uuid(self.id),
            revision: model::SubjectRevision::new(self.revision)?,
            values_digest: digest(&self.values_digest)?,
        })
    }
}
impl From<model::SubjectRevisionRef> for SubjectRef {
    fn from(value: model::SubjectRevisionRef) -> Self {
        Self {
            id: value.id.as_uuid(),
            revision: value.revision.get(),
            values_digest: value.values_digest.to_hex(),
        }
    }
}

pub(super) fn text<const MAX: usize>(value: String) -> Result<model::ParticipantText<MAX>, Error> {
    Ok(model::ParticipantText::new(&value)?)
}
pub(super) fn digest(value: &str) -> Result<Sha256Digest, Error> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidInput(
            "digest must be lowercase SHA-256 hexadecimal".into(),
        ));
    }
    Ok(Sha256Digest::from_hex(value)?)
}
