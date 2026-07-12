//! Authentication use cases: credential hashing, hashing-cost calibration,
//! second-factor enrollment, and one-time-code verification.

pub mod password;
pub mod totp;

pub use password::{
    CalibratePasswordHashing, CalibrationReport, CalibrationVerdict, HashPassword, VerifyPassword,
    DEFAULT_BAND_MAX_MS, DEFAULT_BAND_MIN_MS,
};
pub use totp::{EnrollTotp, TotpEnrollmentBundle, VerifyTotp};
