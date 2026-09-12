//! Single-use recovery codes for account access without the second factor.
//!
//! At enrollment a fixed number of random, human-typeable codes is issued.
//! The plain codes are shown to the user exactly once; only their PHC
//! hashes are kept. Each code grants access a single time: a successful
//! match consumes its slot.
//!
//! Plain codes travel in `Zeroizing` buffers so they are wiped on drop; see
//! docs/adr/0003-zeroize-secret-material-in-domain.md.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::crypto::password::{PasswordHasher, PasswordVerification};
use crate::error::DomainError;

/// Number of recovery codes issued at enrollment.
pub const RECOVERY_CODE_COUNT: usize = 8;

/// Result of presenting a candidate recovery code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryCodeOutcome {
    /// The code matched an unused slot; that slot is now consumed.
    Accepted,
    /// The code matched no unused slot.
    Rejected,
}

/// Outbound port: issues fresh random recovery codes in plain form.
pub trait RecoveryCodeGenerator {
    /// Produces `count` random, human-typeable codes.
    fn generate(&self, count: usize) -> Result<Vec<Zeroizing<String>>, DomainError>;
}

/// The stored form of a user's recovery codes: one PHC hash per issued
/// code, where a consumed slot is cleared and can never match again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredRecoveryCodeSet")]
pub struct RecoveryCodeSet {
    /// One slot per issued code; `None` once the code has been used.
    slots: Vec<Option<String>>,
}

#[derive(Deserialize)]
struct StoredRecoveryCodeSet {
    slots: Vec<Option<String>>,
}

impl TryFrom<StoredRecoveryCodeSet> for RecoveryCodeSet {
    type Error = DomainError;

    fn try_from(stored: StoredRecoveryCodeSet) -> Result<Self, Self::Error> {
        Self::from_slots(stored.slots)
    }
}

impl RecoveryCodeSet {
    /// Builds the set from the PHC hashes of exactly
    /// [`RECOVERY_CODE_COUNT`] plain codes.
    pub fn from_hashes(hashes: Vec<String>) -> Result<Self, DomainError> {
        Self::from_slots(hashes.into_iter().map(Some).collect())
    }

    fn from_slots(slots: Vec<Option<String>>) -> Result<Self, DomainError> {
        if slots.len() != RECOVERY_CODE_COUNT {
            return Err(DomainError::InvalidRecoveryCodeCount {
                expected: RECOVERY_CODE_COUNT,
                actual: slots.len(),
            });
        }
        Ok(Self { slots })
    }

    /// Number of codes that have not been used yet.
    pub fn remaining(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    /// Checks a candidate code against every unused slot and consumes the
    /// slot that matches. Every stored hash is expected to be well formed;
    /// a malformed one surfaces as an error, never as a rejection.
    pub fn consume<H>(&mut self, code: &str, hasher: &H) -> Result<RecoveryCodeOutcome, DomainError>
    where
        H: PasswordHasher + ?Sized,
    {
        for slot in &mut self.slots {
            let Some(hash) = slot else { continue };
            match hasher.verify(code, hash)? {
                PasswordVerification::Match => {
                    *slot = None;
                    return Ok(RecoveryCodeOutcome::Accepted);
                }
                PasswordVerification::Mismatch => {}
            }
        }
        Ok(RecoveryCodeOutcome::Rejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test double that "hashes" by prefixing, so matches are exact string
    /// comparisons and a designated stored value counts as malformed.
    struct FakeHasher;

    const MALFORMED: &str = "malformed";

    impl PasswordHasher for FakeHasher {
        fn hash(&self, password: &str) -> Result<String, DomainError> {
            Ok(format!("fake:{password}"))
        }

        fn verify(
            &self,
            password: &str,
            stored_phc: &str,
        ) -> Result<PasswordVerification, DomainError> {
            if stored_phc == MALFORMED {
                return Err(DomainError::MalformedPasswordHash);
            }
            if stored_phc == format!("fake:{password}") {
                Ok(PasswordVerification::Match)
            } else {
                Ok(PasswordVerification::Mismatch)
            }
        }
    }

    fn issued_set() -> RecoveryCodeSet {
        let hashes = (0..RECOVERY_CODE_COUNT)
            .map(|i| format!("fake:code-{i}"))
            .collect();
        RecoveryCodeSet::from_hashes(hashes).unwrap()
    }

    #[test]
    fn a_full_set_holds_the_issued_count() {
        assert_eq!(issued_set().remaining(), RECOVERY_CODE_COUNT);
    }

    #[test]
    fn building_from_the_wrong_number_of_hashes_is_rejected() {
        let err = RecoveryCodeSet::from_hashes(vec!["fake:only-one".to_string()]).unwrap_err();
        assert_eq!(
            err,
            DomainError::InvalidRecoveryCodeCount {
                expected: RECOVERY_CODE_COUNT,
                actual: 1,
            }
        );
    }

    #[test]
    fn a_code_is_accepted_once_and_rejected_on_reuse() {
        let mut set = issued_set();
        assert_eq!(
            set.consume("code-3", &FakeHasher).unwrap(),
            RecoveryCodeOutcome::Accepted
        );
        assert_eq!(set.remaining(), RECOVERY_CODE_COUNT - 1);
        assert_eq!(
            set.consume("code-3", &FakeHasher).unwrap(),
            RecoveryCodeOutcome::Rejected
        );
        assert_eq!(set.remaining(), RECOVERY_CODE_COUNT - 1);
    }

    #[test]
    fn an_unknown_code_is_rejected_and_consumes_nothing() {
        let mut set = issued_set();
        assert_eq!(
            set.consume("never-issued", &FakeHasher).unwrap(),
            RecoveryCodeOutcome::Rejected
        );
        assert_eq!(set.remaining(), RECOVERY_CODE_COUNT);
    }

    #[test]
    fn each_code_works_exactly_once_until_the_set_is_exhausted() {
        let mut set = issued_set();
        for i in 0..RECOVERY_CODE_COUNT {
            assert_eq!(
                set.consume(&format!("code-{i}"), &FakeHasher).unwrap(),
                RecoveryCodeOutcome::Accepted
            );
        }
        assert_eq!(set.remaining(), 0);
        assert_eq!(
            set.consume("code-0", &FakeHasher).unwrap(),
            RecoveryCodeOutcome::Rejected
        );
    }

    #[test]
    fn a_malformed_stored_hash_surfaces_as_an_error() {
        let mut hashes: Vec<String> = (0..RECOVERY_CODE_COUNT)
            .map(|i| format!("fake:code-{i}"))
            .collect();
        hashes[0] = MALFORMED.to_string();
        let mut set = RecoveryCodeSet::from_hashes(hashes).unwrap();
        assert_eq!(
            set.consume("code-5", &FakeHasher).unwrap_err(),
            DomainError::MalformedPasswordHash
        );
    }
}
