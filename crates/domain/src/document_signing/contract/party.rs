//! The `CTRTSIG1` statement one party of a bilateral contract signs.

use serde::Serialize;

use super::{require_exact, uuid, ContractStatementError, PARTY_STATEMENT_LEN};
use crate::{
    crypto::Sha256Digest,
    document_signing::{
        canonical_prefix,
        decode::{field, prefix},
        SignerAccount, SigningBindingMaterial,
    },
};
use uuid::Uuid;

const PARTY_MAGIC: &[u8; 8] = b"CTRTSIG1";
const ZERO: [u8; 32] = [0; 32];

/// The fixed signing order. The firm signer statement chains to the exact
/// Client signature and its time-stamp response; the Client chains to nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ContractPosition {
    Client,
    Firm {
        signature_digest: Sha256Digest,
        token_digest: Sha256Digest,
    },
}

impl ContractPosition {
    /// The position byte: 1 for the Client, 2 for the firm signer.
    pub const fn number(&self) -> u8 {
        match self {
            Self::Client => 1,
            Self::Firm { .. } => 2,
        }
    }

    /// SHA-256 of the position 1 signature, carried only in position 2.
    pub const fn signature_digest(&self) -> Option<Sha256Digest> {
        match self {
            Self::Client => None,
            Self::Firm {
                signature_digest, ..
            } => Some(*signature_digest),
        }
    }

    /// SHA-256 of the position 1 time-stamp response, carried only in position 2.
    pub const fn token_digest(&self) -> Option<Sha256Digest> {
        match self {
            Self::Client => None,
            Self::Firm { token_digest, .. } => Some(*token_digest),
        }
    }
}

/// One party's acceptance of an exact request; it checks no signature, proves
/// no eligibility and recomputes no digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ContractPartyStatement {
    account: SignerAccount,
    material: SigningBindingMaterial,
    operation: Uuid,
    request: Uuid,
    request_digest: Sha256Digest,
    content_digest: Sha256Digest,
    position: ContractPosition,
}

impl ContractPartyStatement {
    /// Rejects nil operation or request identities and an all-zero chain
    /// digest in position 2, where zeros would read as an absent chain.
    pub fn new(
        account: SignerAccount,
        material: SigningBindingMaterial,
        operation: Uuid,
        request: Uuid,
        request_digest: Sha256Digest,
        content_digest: Sha256Digest,
        position: ContractPosition,
    ) -> Result<Self, ContractStatementError> {
        if operation.is_nil() || request.is_nil() {
            return Err(ContractStatementError::InvalidIdentity);
        }
        if let ContractPosition::Firm {
            signature_digest,
            token_digest,
        } = position
        {
            if signature_digest.as_bytes() == &ZERO || token_digest.as_bytes() == &ZERO {
                return Err(ContractStatementError::InvalidPosition);
            }
        }
        Ok(Self {
            account,
            material,
            operation,
            request,
            request_digest,
            content_digest,
            position,
        })
    }

    pub const fn account(&self) -> &SignerAccount {
        &self.account
    }

    pub const fn material(&self) -> &SigningBindingMaterial {
        &self.material
    }

    pub const fn operation(&self) -> Uuid {
        self.operation
    }

    pub const fn request(&self) -> Uuid {
        self.request
    }

    pub const fn request_digest(&self) -> Sha256Digest {
        self.request_digest
    }

    pub const fn content_digest(&self) -> Sha256Digest {
        self.content_digest
    }

    pub const fn position(&self) -> ContractPosition {
        self.position
    }

    /// Encodes the shared prefix with `CTRTSIG1`, then the request context and
    /// the position chain; position 1 writes zeros for both chain digests.
    pub fn canonical_bytes(&self) -> [u8; PARTY_STATEMENT_LEN] {
        let mut bytes = [0; PARTY_STATEMENT_LEN];
        bytes[..142].copy_from_slice(&canonical_prefix(
            PARTY_MAGIC,
            &self.account,
            &self.material,
        ));
        bytes[142..158].copy_from_slice(self.operation.as_bytes());
        bytes[158..174].copy_from_slice(self.request.as_bytes());
        bytes[174..206].copy_from_slice(self.request_digest.as_bytes());
        bytes[206..238].copy_from_slice(self.content_digest.as_bytes());
        bytes[238] = self.position.number();
        if let ContractPosition::Firm {
            signature_digest,
            token_digest,
        } = self.position
        {
            bytes[239..271].copy_from_slice(signature_digest.as_bytes());
            bytes[271..303].copy_from_slice(token_digest.as_bytes());
        }
        bytes
    }

    /// Decodes one exact statement, reapplying every constructor rule; a
    /// position 1 statement with a non-zero chain digest is not canonical.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ContractStatementError> {
        if bytes.len() != PARTY_STATEMENT_LEN
            || &bytes[..8] != PARTY_MAGIC
            || bytes[8..10] != [1, 1]
        {
            return Err(ContractStatementError::StatementMismatch);
        }
        let (account, material) = prefix(bytes, PARTY_MAGIC, PARTY_STATEMENT_LEN)?;
        let position = match bytes[238] {
            1 => ContractPosition::Client,
            2 => ContractPosition::Firm {
                signature_digest: Sha256Digest::from_array(field(bytes, 239)?),
                token_digest: Sha256Digest::from_array(field(bytes, 271)?),
            },
            _ => return Err(ContractStatementError::InvalidPosition),
        };
        let statement = Self::new(
            account,
            material,
            uuid(bytes, 142),
            uuid(bytes, 158),
            Sha256Digest::from_array(field(bytes, 174)?),
            Sha256Digest::from_array(field(bytes, 206)?),
            position,
        )?;
        statement.require_exact_bytes(bytes)?;
        Ok(statement)
    }

    /// Rejects any difference from the reconstructed statement.
    pub fn require_exact_bytes(&self, supplied: &[u8]) -> Result<(), ContractStatementError> {
        require_exact(supplied, &self.canonical_bytes())
    }
}
