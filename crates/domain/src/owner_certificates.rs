//! Structural account-certificate bindings and terminal withdrawal records.
//!
//! These values do not authenticate an account, verify a signature or commit a
//! registration. Rationale: docs/adr/0067-owner-certificate-bindings.md.

use uuid::Uuid;

use crate::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
};

/// Structural rejections contain no submitted identifiers or credential bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BindingError {
    #[error("an active owner account is required")]
    OwnerRequired,
    #[error("certificate binding belongs to another account")]
    DifferentOwner,
    #[error("certificate binding identities must not be nil")]
    InvalidIdentity,
    #[error("credential trust revision must be positive")]
    InvalidTrustRevision,
    #[error("account revision or generation is outside its valid range")]
    InvalidAccountVersion,
    #[error("certificate binding revision differs from the expectation")]
    RevisionConflict,
    #[error("certificate binding is already withdrawn")]
    Withdrawn,
    #[error("account counters precede the registration capture")]
    StaleAccount,
}

/// Checked account facts supplied by the application, not an authentication proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerAccount {
    id: UserId,
    revision: u64,
    generation: u64,
}

impl OwnerAccount {
    /// Restricts supplied facts to an active Owner and persisted counter ranges.
    pub fn new(
        id: UserId,
        role: Role,
        active: bool,
        revision: u64,
        generation: u64,
    ) -> Result<Self, BindingError> {
        if id.as_uuid().is_nil() {
            return Err(BindingError::InvalidIdentity);
        }
        if role != Role::Owner || !active {
            return Err(BindingError::OwnerRequired);
        }
        if revision > i64::MAX as u64 || generation > revision {
            return Err(BindingError::InvalidAccountVersion);
        }
        Ok(Self {
            id,
            revision,
            generation,
        })
    }
}

/// Exact public references; cryptographic validation belongs to the verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingMaterial {
    deployment: Uuid,
    binding: Uuid,
    root: Sha256Digest,
    leaf: Sha256Digest,
    trust_revision: u32,
}

impl BindingMaterial {
    /// Accepts opaque fingerprints and exact non-nil identities without generating them.
    pub fn new(
        deployment: Uuid,
        binding: Uuid,
        root: Sha256Digest,
        leaf: Sha256Digest,
        trust_revision: u32,
    ) -> Result<Self, BindingError> {
        if deployment.is_nil() || binding.is_nil() {
            return Err(BindingError::InvalidIdentity);
        }
        if trust_revision == 0 {
            return Err(BindingError::InvalidTrustRevision);
        }
        Ok(Self {
            deployment,
            binding,
            root,
            leaf,
            trust_revision,
        })
    }
}

/// Immutable registration intent for the Owner's own account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingStatement {
    owner: OwnerAccount,
    material: BindingMaterial,
}

impl BindingStatement {
    /// Requires the target account to be the supplied active Owner.
    pub fn new(
        actor: OwnerAccount,
        target: UserId,
        material: BindingMaterial,
    ) -> Result<Self, BindingError> {
        if actor.id != target {
            return Err(BindingError::DifferentOwner);
        }
        Ok(Self {
            owner: actor,
            material,
        })
    }

    /// Encodes registration purpose 1 and the exact absent-to-revision-1 transition.
    pub fn canonical_bytes(&self) -> [u8; 150] {
        canonical(&self.owner, &self.material, 1, 0, 1)
    }
}

/// Withdrawal intent with original material and supplied current account facts.
///
/// This value requires no new private-key signature or trust validity assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingWithdrawal {
    owner: OwnerAccount,
    material: BindingMaterial,
}

impl BindingWithdrawal {
    /// Encodes withdrawal purpose 2 and the terminal revision-1-to-2 transition.
    pub fn canonical_bytes(&self) -> [u8; 150] {
        canonical(&self.owner, &self.material, 2, 1, 2)
    }
}

/// Local binding history; construction does not prove external verification or persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingRecord {
    registration: BindingStatement,
    withdrawal: Option<BindingWithdrawal>,
}

impl BindingRecord {
    /// Models a registration supplied by the caller without validating its signature.
    pub fn registered(registration: BindingStatement) -> Self {
        Self {
            registration,
            withdrawal: None,
        }
    }

    /// Returns the only live revision or its terminal withdrawal revision.
    pub const fn revision(&self) -> u32 {
        if self.withdrawal.is_some() {
            2
        } else {
            1
        }
    }

    /// Preserves the exact registration intent, including after withdrawal.
    pub const fn registration(&self) -> &BindingStatement {
        &self.registration
    }

    /// Returns the terminal withdrawal intent when one has been applied locally.
    pub const fn withdrawal(&self) -> Option<&BindingWithdrawal> {
        self.withdrawal.as_ref()
    }

    /// Withdraws once for the same Owner with nondecreasing account counters.
    pub fn withdraw(
        &self,
        actor: OwnerAccount,
        expected_revision: u32,
    ) -> Result<Self, BindingError> {
        let captured = &self.registration.owner;
        if actor.id != captured.id {
            return Err(BindingError::DifferentOwner);
        }
        if self.withdrawal.is_some() {
            return Err(BindingError::Withdrawn);
        }
        if expected_revision != 1 {
            return Err(BindingError::RevisionConflict);
        }
        if actor.revision < captured.revision || actor.generation < captured.generation {
            return Err(BindingError::StaleAccount);
        }
        Ok(Self {
            registration: self.registration.clone(),
            withdrawal: Some(BindingWithdrawal {
                owner: actor,
                material: self.registration.material,
            }),
        })
    }
}

fn canonical(
    owner: &OwnerAccount,
    material: &BindingMaterial,
    purpose: u8,
    expected: u32,
    proposed: u32,
) -> [u8; 150] {
    let mut bytes = [0; 150];
    bytes[..8].copy_from_slice(b"OWNCERT1");
    bytes[8] = purpose;
    bytes[9] = 1;
    bytes[10..26].copy_from_slice(material.deployment.as_bytes());
    bytes[26..58].copy_from_slice(material.root.as_bytes());
    bytes[58..62].copy_from_slice(&material.trust_revision.to_be_bytes());
    bytes[62..78].copy_from_slice(owner.id.as_uuid().as_bytes());
    bytes[78..86].copy_from_slice(&owner.revision.to_be_bytes());
    bytes[86..94].copy_from_slice(&owner.generation.to_be_bytes());
    bytes[94..110].copy_from_slice(material.binding.as_bytes());
    bytes[110..114].copy_from_slice(&expected.to_be_bytes());
    bytes[114..118].copy_from_slice(&proposed.to_be_bytes());
    bytes[118..].copy_from_slice(material.leaf.as_bytes());
    bytes
}
