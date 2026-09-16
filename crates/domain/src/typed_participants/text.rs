use crate::DomainError;

/// Bounded declared text, without normalization of interior Unicode forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantText<const MAX: usize>(String);

impl<const MAX: usize> ParticipantText<MAX> {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        if value.chars().any(char::is_control) {
            return Err(invalid("control characters"));
        }
        let value = value.trim();
        if value.is_empty() || value.chars().count() > MAX {
            return Err(invalid("text length"));
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An explicit explanation for absent knowledge, not an implicit null.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantReason(ParticipantText<500>);
impl ParticipantReason {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        ParticipantText::new(value).map(Self)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declared<T> {
    Known(T),
    Unknown(ParticipantReason),
}

/// Checks the declared identifier's shape, without establishing civil identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curp(String);
impl Curp {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let value = ParticipantText::<18>::new(value)?;
        let value = value.as_str();
        if value.len() != 18 || !value.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(invalid("curp"));
        }
        Ok(Self(value.to_ascii_uppercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A professional identifier declared by staff, not external accreditation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfessionalLicense {
    number: String,
    issuer: ParticipantText<200>,
}
impl ProfessionalLicense {
    pub fn new(number: &str, issuer: &str) -> Result<Self, DomainError> {
        let number = ParticipantText::<32>::new(number)?;
        if !number.as_str().bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(invalid("license number"));
        }
        Ok(Self {
            number: number.as_str().to_owned(),
            issuer: ParticipantText::new(issuer)?,
        })
    }
    pub fn number(&self) -> &str {
        &self.number
    }
    pub fn issuer(&self) -> &str {
        self.issuer.as_str()
    }
}

pub(super) fn optional<const MAX: usize>(
    value: Option<&str>,
) -> Result<Option<ParticipantText<MAX>>, DomainError> {
    match value {
        Some(value) if value.chars().any(char::is_control) => Err(invalid("control characters")),
        Some(value) if !value.trim().is_empty() => ParticipantText::new(value).map(Some),
        _ => Ok(None),
    }
}
pub(super) fn invalid(field: &'static str) -> DomainError {
    DomainError::InvalidTypedParticipantValue(field)
}
