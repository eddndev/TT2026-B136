//! Exact public intentions for individual document signing and certificate binding.
//!
//! These structural values neither authenticate an account nor verify possession.
//! Rationale: docs/adr/0074-individual-document-signatures.md.

mod consent;
mod decode;

pub use consent::DocumentConsent;
pub use uuid::Uuid;

use serde::Serialize;

use crate::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
};

/// Structural rejections expose no submitted identifiers or credential material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SigningError {
    #[error("an active litigator account is required")]
    LitigatorRequired,
    #[error("certificate binding belongs to another signing account")]
    DifferentAccount,
    #[error("individual signing identities must not be nil")]
    InvalidIdentity,
    #[error("credential trust revision must be positive")]
    InvalidTrustRevision,
    #[error("account revision or generation is outside its valid range")]
    InvalidAccountVersion,
    #[error("submitted bytes differ from the exact signing intention")]
    StatementMismatch,
}

/// Supplied active Litigator facts; constructing this value proves no authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SignerAccount {
    id: UserId,
    revision: u64,
    generation: u64,
}

impl SignerAccount {
    /// Requires an active Litigator and the durable account counter ranges.
    pub fn new(
        id: UserId,
        role: Role,
        active: bool,
        revision: u64,
        generation: u64,
    ) -> Result<Self, SigningError> {
        if id.as_uuid().is_nil() {
            return Err(SigningError::InvalidIdentity);
        }
        if role != Role::Litigator || !active {
            return Err(SigningError::LitigatorRequired);
        }
        if revision > i64::MAX as u64 || generation > revision {
            return Err(SigningError::InvalidAccountVersion);
        }
        Ok(Self {
            id,
            revision,
            generation,
        })
    }

    pub const fn id(&self) -> UserId {
        self.id
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// Public references supplied for an exact binding and published trust revision.
/// Fingerprints are opaque here; certificate inspection belongs to the verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SigningBindingMaterial {
    deployment: Uuid,
    binding: Uuid,
    root: Sha256Digest,
    leaf: Sha256Digest,
    trust_revision: u32,
}

impl SigningBindingMaterial {
    pub fn new(
        deployment: Uuid,
        binding: Uuid,
        root: Sha256Digest,
        leaf: Sha256Digest,
        trust_revision: u32,
    ) -> Result<Self, SigningError> {
        if deployment.is_nil() || binding.is_nil() {
            return Err(SigningError::InvalidIdentity);
        }
        if trust_revision == 0 {
            return Err(SigningError::InvalidTrustRevision);
        }
        Ok(Self {
            deployment,
            binding,
            root,
            leaf,
            trust_revision,
        })
    }

    pub const fn deployment(&self) -> Uuid {
        self.deployment
    }

    pub const fn binding(&self) -> Uuid {
        self.binding
    }

    pub const fn root(&self) -> Sha256Digest {
        self.root
    }

    pub const fn leaf(&self) -> Sha256Digest {
        self.leaf
    }

    pub const fn trust_revision(&self) -> u32 {
        self.trust_revision
    }
}

/// Registration intention for the Litigator's own public signing certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SigningBindingStatement {
    account: SignerAccount,
    material: SigningBindingMaterial,
}

impl SigningBindingStatement {
    pub fn new(
        account: SignerAccount,
        target: UserId,
        material: SigningBindingMaterial,
    ) -> Result<Self, SigningError> {
        if target.as_uuid().is_nil() {
            return Err(SigningError::InvalidIdentity);
        }
        if account.id != target {
            return Err(SigningError::DifferentAccount);
        }
        Ok(Self { account, material })
    }

    pub const fn account(&self) -> &SignerAccount {
        &self.account
    }

    pub const fn material(&self) -> &SigningBindingMaterial {
        &self.material
    }

    /// Encodes registration purpose 1 and format version 1 with fixed-width fields.
    pub fn canonical_bytes(&self) -> [u8; 142] {
        canonical_prefix(b"DOCBIND1", &self.account, &self.material)
    }

    /// Rejects any difference from a reconstructed intention; this checks no signature.
    pub fn require_exact_bytes(&self, supplied: &[u8]) -> Result<(), SigningError> {
        require_exact(supplied, &self.canonical_bytes())
    }
}

fn canonical_prefix(
    purpose: &[u8; 8],
    account: &SignerAccount,
    material: &SigningBindingMaterial,
) -> [u8; 142] {
    let mut bytes = [0; 142];
    bytes[..8].copy_from_slice(purpose);
    bytes[8] = 1;
    bytes[9] = 1;
    bytes[10..26].copy_from_slice(material.deployment.as_bytes());
    bytes[26..58].copy_from_slice(material.root.as_bytes());
    bytes[58..62].copy_from_slice(&material.trust_revision.to_be_bytes());
    bytes[62..78].copy_from_slice(account.id.as_uuid().as_bytes());
    bytes[78..86].copy_from_slice(&account.revision.to_be_bytes());
    bytes[86..94].copy_from_slice(&account.generation.to_be_bytes());
    bytes[94..110].copy_from_slice(material.binding.as_bytes());
    bytes[110..142].copy_from_slice(material.leaf.as_bytes());
    bytes
}

fn require_exact(supplied: &[u8], expected: &[u8]) -> Result<(), SigningError> {
    if supplied != expected {
        return Err(SigningError::StatementMismatch);
    }
    Ok(())
}
