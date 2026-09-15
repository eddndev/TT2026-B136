use crate::cases::CaseMetadata;

use super::{CaseAdministrativeStatus, PenalCaseProfile};

/// Replaceable fields exclude administrative status and recorded stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseEditableValues {
    metadata: CaseMetadata,
    profile: Option<PenalCaseProfile>,
}
impl CaseEditableValues {
    pub fn new(metadata: CaseMetadata, profile: Option<PenalCaseProfile>) -> Self {
        Self { metadata, profile }
    }
    pub fn metadata(&self) -> &CaseMetadata {
        &self.metadata
    }
    pub fn profile(&self) -> Option<&PenalCaseProfile> {
        self.profile.as_ref()
    }
}

/// Complete creation cannot omit a profile or choose a closed state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenalCaseCreation {
    metadata: CaseMetadata,
    profile: PenalCaseProfile,
}
impl PenalCaseCreation {
    pub fn new(metadata: CaseMetadata, profile: PenalCaseProfile) -> Self {
        Self { metadata, profile }
    }
    pub fn metadata(&self) -> &CaseMetadata {
        &self.metadata
    }
    pub fn profile(&self) -> &PenalCaseProfile {
        &self.profile
    }
    pub fn into_values(self) -> CaseAdministrationValues {
        CaseAdministrationValues::new(
            CaseEditableValues::new(self.metadata, Some(self.profile)),
            CaseAdministrativeStatus::Active,
        )
    }
}

/// Immutable administrative values; provenance and stage remain independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationValues {
    editable: CaseEditableValues,
    status: CaseAdministrativeStatus,
}
impl CaseAdministrationValues {
    pub fn new(editable: CaseEditableValues, status: CaseAdministrativeStatus) -> Self {
        Self { editable, status }
    }
    pub fn basic(metadata: CaseMetadata) -> Self {
        Self::new(
            CaseEditableValues::new(metadata, None),
            CaseAdministrativeStatus::Active,
        )
    }
    pub fn editable(&self) -> &CaseEditableValues {
        &self.editable
    }
    pub fn metadata(&self) -> &CaseMetadata {
        self.editable.metadata()
    }
    pub fn profile(&self) -> Option<&PenalCaseProfile> {
        self.editable.profile()
    }
    pub const fn status(&self) -> CaseAdministrativeStatus {
        self.status
    }
    pub fn with_status(&self, status: CaseAdministrativeStatus) -> Self {
        Self::new(self.editable.clone(), status)
    }

    /// CADM1 uses UTF-8 byte lengths and preserves the order of offenses.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"CADM1".to_vec();
        bytes.push(match self.status {
            CaseAdministrativeStatus::Active => 0,
            CaseAdministrativeStatus::Closed => 1,
        });
        encode_text(&mut bytes, self.metadata().title());
        encode_text(&mut bytes, self.metadata().reference());
        let Some(profile) = self.profile() else {
            bytes.push(0);
            return bytes;
        };
        bytes.push(1);
        for value in [
            profile.nuc(),
            profile.nuc_authority(),
            profile.judicial_case_number(),
            profile.judicial_authority(),
        ] {
            encode_text(&mut bytes, value);
        }
        bytes.extend_from_slice(&(profile.offenses().len() as u32).to_be_bytes());
        for offense in profile.offenses() {
            encode_text(&mut bytes, offense)
        }
        encode_optional(&mut bytes, profile.general_information());
        encode_optional(&mut bytes, profile.complementary_identifiers());
        bytes
    }
}

fn encode_text(bytes: &mut Vec<u8>, value: &str) {
    // Validated scalar limits keep every UTF-8 field far below u32::MAX bytes.
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
fn encode_optional(bytes: &mut Vec<u8>, value: Option<&str>) {
    if let Some(value) = value {
        bytes.push(1);
        encode_text(bytes, value)
    } else {
        bytes.push(0)
    }
}
