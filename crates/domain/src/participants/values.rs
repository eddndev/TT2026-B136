use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::DomainError;

/// Organizational visibility in the directory, without legal-state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryStatus {
    Active,
    Archived,
}

impl DirectoryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

impl FromStr for DirectoryStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            other => Err(DomainError::InvalidDirectoryStatus(other.to_owned())),
        }
    }
}

/// Manually recorded text; names and roles do not establish verified identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParticipantValues {
    display_name: String,
    procedural_role: String,
    organization: Option<String>,
    legal_status: Option<String>,
    directory_status: DirectoryStatus,
}

impl ParticipantValues {
    pub fn new(
        display_name: &str,
        procedural_role: &str,
        organization: Option<&str>,
        legal_status: Option<&str>,
        directory_status: DirectoryStatus,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            display_name: required(display_name, "display_name", 200)?,
            procedural_role: required(procedural_role, "procedural_role", 80)?,
            organization: optional(organization, "organization", 200)?,
            legal_status: optional(legal_status, "legal_status", 160)?,
            directory_status,
        })
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn procedural_role(&self) -> &str {
        &self.procedural_role
    }

    pub fn organization(&self) -> Option<&str> {
        self.organization.as_deref()
    }

    pub fn legal_status(&self) -> Option<&str> {
        self.legal_status.as_deref()
    }

    pub const fn directory_status(&self) -> DirectoryStatus {
        self.directory_status
    }

    /// Changes only the organizational state of these already validated values.
    pub fn with_directory_status(&self, status: DirectoryStatus) -> Self {
        Self {
            directory_status: status,
            ..self.clone()
        }
    }

    /// PART1: length-prefixed UTF-8 values, optional markers and one state byte.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"PART1".to_vec();
        write_value(&mut bytes, self.display_name());
        write_value(&mut bytes, self.procedural_role());
        for value in [self.organization(), self.legal_status()] {
            match value {
                Some(value) => {
                    bytes.push(1);
                    write_value(&mut bytes, value);
                }
                None => bytes.push(0),
            }
        }
        bytes.push(match self.directory_status {
            DirectoryStatus::Active => 0,
            DirectoryStatus::Archived => 1,
        });
        bytes
    }
}

fn normalized(value: &str, field: &'static str, limit: usize) -> Result<String, DomainError> {
    if value.chars().any(char::is_control) {
        return Err(invalid(field, "control characters are not allowed"));
    }
    let value = value.trim();
    if value.chars().count() > limit {
        return Err(invalid(field, "character limit exceeded"));
    }
    Ok(value.to_owned())
}

fn required(value: &str, field: &'static str, limit: usize) -> Result<String, DomainError> {
    let value = normalized(value, field, limit)?;
    if value.is_empty() {
        return Err(invalid(field, "must not be empty"));
    }
    Ok(value)
}

fn optional(
    value: Option<&str>,
    field: &'static str,
    limit: usize,
) -> Result<Option<String>, DomainError> {
    value
        .map(|value| normalized(value, field, limit))
        .transpose()
        .map(|value| value.filter(|value| !value.is_empty()))
}

fn invalid(field: &'static str, reason: &'static str) -> DomainError {
    DomainError::InvalidParticipantValues { field, reason }
}

fn write_value(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
