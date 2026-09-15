use crate::DomainError;

use super::text::{invalid, optional, required};

/// Complete manually registered identifiers, authorities and descriptions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenalCaseProfile {
    nuc: String,
    nuc_authority: String,
    judicial_case_number: String,
    judicial_authority: String,
    offenses: Vec<String>,
    general_information: Option<String>,
    complementary_identifiers: Option<String>,
}

impl PenalCaseProfile {
    pub fn new(
        nuc: &str,
        nuc_authority: &str,
        judicial_case_number: &str,
        judicial_authority: &str,
        offenses: &[&str],
        general_information: Option<&str>,
        complementary_identifiers: Option<&str>,
    ) -> Result<Self, DomainError> {
        if !(1..=8).contains(&offenses.len()) {
            return Err(invalid("offenses", "must contain between 1 and 8 entries"));
        }
        let mut normalized = Vec::with_capacity(offenses.len());
        for offense in offenses {
            let value = required(offense, "offenses", 120)?;
            if normalized.contains(&value) {
                return Err(invalid("offenses", "duplicate entries are not allowed"));
            }
            normalized.push(value);
        }
        Ok(Self {
            nuc: required(nuc, "nuc", 100)?,
            nuc_authority: required(nuc_authority, "nuc_authority", 200)?,
            judicial_case_number: required(judicial_case_number, "judicial_case_number", 100)?,
            judicial_authority: required(judicial_authority, "judicial_authority", 200)?,
            offenses: normalized,
            general_information: optional(general_information, "general_information", 1000, true)?,
            complementary_identifiers: optional(
                complementary_identifiers,
                "complementary_identifiers",
                300,
                false,
            )?,
        })
    }

    pub fn nuc(&self) -> &str {
        &self.nuc
    }
    pub fn nuc_authority(&self) -> &str {
        &self.nuc_authority
    }
    pub fn judicial_case_number(&self) -> &str {
        &self.judicial_case_number
    }
    pub fn judicial_authority(&self) -> &str {
        &self.judicial_authority
    }
    pub fn offenses(&self) -> &[String] {
        &self.offenses
    }
    pub fn general_information(&self) -> Option<&str> {
        self.general_information.as_deref()
    }
    pub fn complementary_identifiers(&self) -> Option<&str> {
        self.complementary_identifiers.as_deref()
    }
}
