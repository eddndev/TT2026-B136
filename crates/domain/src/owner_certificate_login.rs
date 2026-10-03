//! Bounded, domain-separated Owner certificate first-factor statements.
//!
//! Construction validates structure only. Account/binding ownership, current
//! trust, certificate admission, nonce freshness and mandatory MFA belong to
//! application workflows and their adapters. No private key or IO is involved.

use crate::{
    identity::{Role, UserId},
    owner_certificates::BindingMaterial,
};

const MAX_LIFETIME_SECONDS: i64 = 300;

/// Structural rejections disclose no submitted identity or proof material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LoginError {
    #[error("an active owner account is required")]
    OwnerRequired,
    #[error("owner login identity must not be nil")]
    InvalidIdentity,
    #[error("authentication generation is outside its persisted range")]
    InvalidGeneration,
    #[error("owner login nonce must contain exactly 32 bytes")]
    InvalidNonceLength,
    #[error("owner login window must contain 1 to 300 nonnegative Unix seconds")]
    InvalidTimeWindow,
}

/// Supplied active Owner facts; this value does not authenticate an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginAccount {
    id: UserId,
    generation: u64,
}

impl LoginAccount {
    /// Checks the identity, role, activity and PostgreSQL generation range.
    pub fn new(id: UserId, role: Role, active: bool, generation: u64) -> Result<Self, LoginError> {
        if id.as_uuid().is_nil() {
            return Err(LoginError::InvalidIdentity);
        }
        if role != Role::Owner || !active {
            return Err(LoginError::OwnerRequired);
        }
        if generation > i64::MAX as u64 {
            return Err(LoginError::InvalidGeneration);
        }
        Ok(Self { id, generation })
    }

    /// Returns the exact intended account identity.
    pub const fn id(&self) -> UserId {
        self.id
    }

    /// Returns the captured revocation generation without incrementing it.
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// Owned public nonce bytes; construction neither generates nor proves entropy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginNonce([u8; 32]);

impl LoginNonce {
    /// Copies exactly 32 bytes, including any structurally valid byte pattern.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoginError> {
        bytes
            .try_into()
            .map(Self)
            .map_err(|_| LoginError::InvalidNonceLength)
    }

    /// Returns the captured nonce without exposing mutable statement state.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Immutable first-factor intent requiring a separate MFA step after proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginStatement {
    owner: LoginAccount,
    material: BindingMaterial,
    nonce: LoginNonce,
    issued_at: i64,
    expires_at: i64,
}

impl LoginStatement {
    /// Requires a nonnegative window lasting from one through 300 seconds.
    /// The expiry is exclusive and must also be capped by current authority.
    pub fn new(
        owner: LoginAccount,
        material: BindingMaterial,
        nonce: LoginNonce,
        issued_at: i64,
        expires_at: i64,
    ) -> Result<Self, LoginError> {
        if issued_at < 0
            || expires_at
                .checked_sub(issued_at)
                .filter(|duration| (1..=MAX_LIFETIME_SECONDS).contains(duration))
                .is_none()
        {
            return Err(LoginError::InvalidTimeWindow);
        }
        Ok(Self {
            owner,
            material,
            nonce,
            issued_at,
            expires_at,
        })
    }

    /// Returns the supplied Owner identity and authentication generation.
    pub const fn owner(&self) -> &LoginAccount {
        &self.owner
    }

    /// Returns the exact intended binding, deployment and trust references.
    pub const fn material(&self) -> &BindingMaterial {
        &self.material
    }

    /// Returns the nonce committed by the statement.
    pub const fn nonce(&self) -> &LoginNonce {
        &self.nonce
    }

    /// Returns the inclusive issue instant in whole Unix seconds.
    pub const fn issued_at_unix_seconds(&self) -> i64 {
        self.issued_at
    }

    /// Returns the exclusive expiry instant in whole Unix seconds.
    pub const fn expires_at_unix_seconds(&self) -> i64 {
        self.expires_at
    }

    /// Checks only the captured window, not live binding or certificate validity.
    pub const fn is_live_at(&self, now: i64) -> bool {
        now >= self.issued_at && now < self.expires_at
    }

    /// Encodes version 1 and login-before-MFA purpose 1 with big-endian integers.
    /// UUIDs and fingerprints retain their raw bytes; no receipt is embedded.
    pub fn canonical_bytes(&self) -> [u8; 182] {
        let mut bytes = [0; 182];
        bytes[..8].copy_from_slice(b"OWNAUTH1");
        bytes[8] = 1;
        bytes[9] = 1;
        bytes[10..26].copy_from_slice(self.material.deployment().as_bytes());
        bytes[26..58].copy_from_slice(self.material.root().as_bytes());
        bytes[58..62].copy_from_slice(&self.material.trust_revision().to_be_bytes());
        bytes[62..78].copy_from_slice(self.owner.id().as_uuid().as_bytes());
        bytes[78..86].copy_from_slice(&self.owner.generation().to_be_bytes());
        bytes[86..102].copy_from_slice(self.material.binding().as_bytes());
        bytes[102..134].copy_from_slice(self.material.leaf().as_bytes());
        bytes[134..166].copy_from_slice(self.nonce.as_bytes());
        bytes[166..174].copy_from_slice(&self.issued_at.to_be_bytes());
        bytes[174..182].copy_from_slice(&self.expires_at.to_be_bytes());
        bytes
    }
}
