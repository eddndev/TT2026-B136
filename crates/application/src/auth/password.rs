//! Use cases around one-way credential hashing.

use std::num::NonZeroU32;
use std::time::Instant;

use domain::crypto::password::{PasswordHasher, PasswordVerification};

use crate::error::ApplicationError;

/// Lower edge, in milliseconds, of the default calibration target band.
pub const DEFAULT_BAND_MIN_MS: u64 = 500;

/// Upper edge, in milliseconds, of the default calibration target band.
pub const DEFAULT_BAND_MAX_MS: u64 = 1000;

/// Fixed input hashed repeatedly during calibration. Its content is
/// irrelevant; only the hashing cost is measured.
const CALIBRATION_SAMPLE: &str = "calibration sample password";

/// Hashes a credential for storage.
pub struct HashPassword<H: PasswordHasher> {
    hasher: H,
}

impl<H: PasswordHasher> HashPassword<H> {
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }

    /// Returns the PHC string to store for this credential.
    pub fn execute(&self, password: &str) -> Result<String, ApplicationError> {
        Ok(self.hasher.hash(password)?)
    }
}

/// Checks a candidate credential against a stored PHC string.
pub struct VerifyPassword<H: PasswordHasher> {
    hasher: H,
}

impl<H: PasswordHasher> VerifyPassword<H> {
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }

    /// A mismatch is a normal outcome; a malformed stored hash is an error.
    pub fn execute(
        &self,
        password: &str,
        stored_phc: &str,
    ) -> Result<PasswordVerification, ApplicationError> {
        Ok(self.hasher.verify(password, stored_phc)?)
    }
}

/// Where a measured mean hashing time falls relative to the target band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationVerdict {
    /// Hashing is faster than the band; parameters are too weak for this
    /// hardware.
    BelowBand,
    /// Hashing lands inside the band (edges included).
    InBand,
    /// Hashing is slower than the band; logins would feel sluggish.
    AboveBand,
}

impl CalibrationVerdict {
    /// Classifies a mean against a band. Both edges are inclusive.
    pub fn classify(mean_ms: f64, band_min_ms: u64, band_max_ms: u64) -> Self {
        if mean_ms < band_min_ms as f64 {
            Self::BelowBand
        } else if mean_ms > band_max_ms as f64 {
            Self::AboveBand
        } else {
            Self::InBand
        }
    }
}

/// Outcome of a calibration run.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationReport {
    /// Number of hash operations measured.
    pub runs: u32,
    /// Mean duration of one hash operation, in milliseconds.
    pub mean_ms: f64,
    /// Lower edge of the target band, in milliseconds.
    pub band_min_ms: u64,
    /// Upper edge of the target band, in milliseconds.
    pub band_max_ms: u64,
    /// Position of the mean relative to the band.
    pub verdict: CalibrationVerdict,
}

/// Measures the wall-clock cost of the configured hashing parameters on
/// the current hardware.
pub struct CalibratePasswordHashing<H: PasswordHasher> {
    hasher: H,
}

impl<H: PasswordHasher> CalibratePasswordHashing<H> {
    pub fn new(hasher: H) -> Self {
        Self { hasher }
    }

    /// Runs `runs` hash operations and reports the mean duration and where
    /// it falls relative to the inclusive target band.
    pub fn execute(
        &self,
        runs: NonZeroU32,
        band_min_ms: u64,
        band_max_ms: u64,
    ) -> Result<CalibrationReport, ApplicationError> {
        let started = Instant::now();
        for _ in 0..runs.get() {
            self.hasher.hash(CALIBRATION_SAMPLE)?;
        }
        let mean_ms = started.elapsed().as_secs_f64() * 1000.0 / f64::from(runs.get());
        Ok(CalibrationReport {
            runs: runs.get(),
            mean_ms,
            band_min_ms,
            band_max_ms,
            verdict: CalibrationVerdict::classify(mean_ms, band_min_ms, band_max_ms),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::DomainError;
    use mockall::mock;

    mock! {
        Hasher {}
        impl PasswordHasher for Hasher {
            fn hash(&self, password: &str) -> Result<String, DomainError>;
            fn verify(
                &self,
                password: &str,
                stored_phc: &str,
            ) -> Result<PasswordVerification, DomainError>;
        }
    }

    #[test]
    fn hash_password_returns_the_ports_phc_string() {
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash()
            .withf(|password| password == "s3cret")
            .times(1)
            .returning(|_| Ok("$argon2id$stub".to_string()));
        let use_case = HashPassword::new(hasher);
        assert_eq!(use_case.execute("s3cret").unwrap(), "$argon2id$stub");
    }

    #[test]
    fn hash_password_surfaces_port_failures_as_domain_errors() {
        let mut hasher = MockHasher::new();
        hasher.expect_hash().returning(|_| {
            Err(DomainError::PasswordHashingFailed(
                "backend down".to_string(),
            ))
        });
        let err = HashPassword::new(hasher).execute("s3cret").unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::PasswordHashingFailed(_))
        ));
    }

    #[test]
    fn verify_password_passes_both_outcomes_through() {
        let mut hasher = MockHasher::new();
        hasher
            .expect_verify()
            .withf(|password, stored| password == "right" && stored == "$phc")
            .returning(|_, _| Ok(PasswordVerification::Match));
        hasher
            .expect_verify()
            .withf(|password, _| password == "wrong")
            .returning(|_, _| Ok(PasswordVerification::Mismatch));
        let use_case = VerifyPassword::new(hasher);
        assert_eq!(
            use_case.execute("right", "$phc").unwrap(),
            PasswordVerification::Match
        );
        assert_eq!(
            use_case.execute("wrong", "$phc").unwrap(),
            PasswordVerification::Mismatch
        );
    }

    #[test]
    fn verify_password_reports_a_malformed_stored_hash_as_an_error() {
        let mut hasher = MockHasher::new();
        hasher
            .expect_verify()
            .returning(|_, _| Err(DomainError::MalformedPasswordHash));
        let err = VerifyPassword::new(hasher)
            .execute("any", "garbage")
            .unwrap_err();
        assert!(matches!(
            err,
            ApplicationError::Domain(DomainError::MalformedPasswordHash)
        ));
    }

    #[test]
    fn calibration_with_an_instant_hasher_reports_below_band() {
        let mut hasher = MockHasher::new();
        hasher
            .expect_hash()
            .times(5)
            .returning(|_| Ok("$argon2id$stub".to_string()));
        let report = CalibratePasswordHashing::new(hasher)
            .execute(
                NonZeroU32::new(5).unwrap(),
                DEFAULT_BAND_MIN_MS,
                DEFAULT_BAND_MAX_MS,
            )
            .unwrap();
        assert_eq!(report.runs, 5);
        assert!(report.mean_ms < DEFAULT_BAND_MIN_MS as f64);
        assert_eq!(report.verdict, CalibrationVerdict::BelowBand);
    }

    #[test]
    fn band_edges_are_inclusive() {
        assert_eq!(
            CalibrationVerdict::classify(500.0, 500, 1000),
            CalibrationVerdict::InBand
        );
        assert_eq!(
            CalibrationVerdict::classify(1000.0, 500, 1000),
            CalibrationVerdict::InBand
        );
        assert_eq!(
            CalibrationVerdict::classify(499.9, 500, 1000),
            CalibrationVerdict::BelowBand
        );
        assert_eq!(
            CalibrationVerdict::classify(1000.1, 500, 1000),
            CalibrationVerdict::AboveBand
        );
    }
}
