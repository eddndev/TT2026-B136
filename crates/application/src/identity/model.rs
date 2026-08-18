//! Values exchanged by identity ports and inbound use cases.

use domain::crypto::RecoveryCodeSet;
use domain::identity::{Role, UserId};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Persisted user material. Sensitive values are encrypted or one-way hashed.
#[derive(Clone)]
pub struct UserRecord {
    pub id: UserId,
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub active: bool,
    pub protected_totp_secret: Vec<u8>,
    pub recovery_codes: RecoveryCodeSet,
    pub revision: u64,
}

/// Identity attached to an authenticated request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub id: UserId,
    pub email: String,
    pub role: Role,
}

impl From<&UserRecord> for Principal {
    fn from(user: &UserRecord) -> Self {
        Self {
            id: user.id,
            email: user.email.clone(),
            role: user.role,
        }
    }
}

/// One-time material returned when an account is enrolled.
pub struct EnrollmentResult {
    pub principal: Principal,
    pub totp_secret_base32: Zeroizing<String>,
    pub otpauth_uri: Zeroizing<String>,
    pub recovery_codes: Vec<Zeroizing<String>>,
}

/// Password-verified challenge awaiting a second factor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginChallenge {
    pub challenge_token: String,
    pub expires_in_seconds: u64,
}

/// Revocable bearer session returned after the second factor succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionResult {
    pub access_token: String,
    pub expires_in_seconds: u64,
    pub principal: Principal,
}
