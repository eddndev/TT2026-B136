//! Human display name of an office account: docs/adr/0087-account-display-names.md.
//!
//! The name is written once by the Owner. It is validated before trimming so
//! that control, format and line-separator characters can never hide inside a
//! value that looks clean once surrounding whitespace is removed. The format
//! table is the Unicode 16.0.0 general category Cf.

use crate::DomainError;

/// Largest accepted display name, counted in Unicode scalar values.
pub const DISPLAY_NAME_MAX_CHARS: usize = 120;

/// Trimmed, single-line account name of 1..=120 Unicode scalar values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

impl DisplayName {
    /// Rejects control and format characters, trims, then checks the length.
    pub fn new(value: &str) -> Result<Self, DomainError> {
        if value.chars().any(rejected) {
            return Err(DomainError::InvalidDisplayName(
                "control or format characters",
            ));
        }
        let trimmed = value.trim();
        if !(1..=DISPLAY_NAME_MAX_CHARS).contains(&trimmed.chars().count()) {
            return Err(DomainError::InvalidDisplayName("length"));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Accepts a persisted value only when it is already in canonical form.
    pub fn from_stored(value: &str) -> Result<Self, DomainError> {
        let name = Self::new(value)?;
        if name.as_str() != value {
            return Err(DomainError::InvalidDisplayName("not canonical"));
        }
        Ok(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

fn rejected(value: char) -> bool {
    value.is_control() || value == '\u{2028}' || value == '\u{2029}' || format(value)
}

fn format(value: char) -> bool {
    let value = u32::from(value);
    FORMAT
        .binary_search_by(|&(start, end)| {
            if end < value {
                std::cmp::Ordering::Less
            } else if start > value {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Inclusive ranges of general category Cf, generated from Unicode 16.0.0.
const FORMAT: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];
