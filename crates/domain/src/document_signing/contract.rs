//! Exact public statements of bilateral contract signing: the `CTRTREQ1`
//! request built by the server and the `CTRTSIG1` statement each party signs.
//!
//! These structural values neither authenticate an account nor verify a
//! signature or a digest. Rationale: docs/adr/0086-bilateral-contract-signing.md.

mod party;

pub use party::{ContractPartyStatement, ContractPosition};

use serde::Serialize;

use super::{decode::field, SigningError};
use crate::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    identity::UserId,
};
use uuid::Uuid;

/// Exact length of a `CTRTREQ1` request.
pub const REQUEST_LEN: usize = 240;
/// Exact length of a `CTRTSIG1` party statement.
pub const PARTY_STATEMENT_LEN: usize = 303;
/// Number of parties carried by every request.
pub const PARTY_COUNT: u8 = 2;
/// Completion rule 1: every party signs before `expires_at` per its time-stamp.
pub const COMPLETION_RULE: u8 = 1;

const REQUEST_MAGIC: &[u8; 8] = b"CTRTREQ1";

/// Structural rejections expose no submitted identifiers or digests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ContractStatementError {
    #[error("contract statement identities must not be nil")]
    InvalidIdentity,
    #[error("the client must differ from the firm signer and the requester")]
    SameParty,
    #[error("both parties must use different certificates")]
    SameCertificate,
    #[error("the request must expire after its creation")]
    InvalidValidity,
    #[error("the party position or its chain digests are invalid")]
    InvalidPosition,
    #[error("submitted bytes differ from the exact contract statement")]
    StatementMismatch,
    #[error(transparent)]
    Signing(#[from] SigningError),
}

/// Every value of one signature request, in `CTRTREQ1` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ContractRequestFields {
    pub deployment: Uuid,
    pub request: Uuid,
    pub case: CaseId,
    pub document: DocumentVersionRef,
    pub content_digest: Sha256Digest,
    pub client: UserId,
    pub firm_signer: UserId,
    pub requester: UserId,
    pub client_leaf: Sha256Digest,
    pub firm_leaf: Sha256Digest,
    pub created_at_seconds: i64,
    pub expires_at_seconds: i64,
}

/// A request whose fields satisfy every codec rule; it proves no eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ContractRequest {
    fields: ContractRequestFields,
}

impl ContractRequest {
    /// Rejects nil identities, a Client equal to another party, equal
    /// certificate leaves and a request that does not expire after creation.
    /// The storage validity window is not a codec rule.
    pub fn new(fields: ContractRequestFields) -> Result<Self, ContractStatementError> {
        let identities = [
            fields.deployment,
            fields.request,
            fields.case.as_uuid(),
            fields.document.id.as_uuid(),
            fields.client.as_uuid(),
            fields.firm_signer.as_uuid(),
            fields.requester.as_uuid(),
        ];
        if identities.iter().any(Uuid::is_nil) {
            return Err(ContractStatementError::InvalidIdentity);
        }
        if fields.client == fields.firm_signer || fields.client == fields.requester {
            return Err(ContractStatementError::SameParty);
        }
        if fields.client_leaf == fields.firm_leaf {
            return Err(ContractStatementError::SameCertificate);
        }
        if fields.created_at_seconds >= fields.expires_at_seconds {
            return Err(ContractStatementError::InvalidValidity);
        }
        Ok(Self { fields })
    }

    pub const fn fields(&self) -> &ContractRequestFields {
        &self.fields
    }

    /// Encodes purpose 1 and format version 1 with fixed-width big-endian fields.
    pub fn canonical_bytes(&self) -> [u8; REQUEST_LEN] {
        let f = &self.fields;
        let mut bytes = [0; REQUEST_LEN];
        bytes[..8].copy_from_slice(REQUEST_MAGIC);
        bytes[8] = 1;
        bytes[9] = 1;
        bytes[10..26].copy_from_slice(f.deployment.as_bytes());
        bytes[26..42].copy_from_slice(f.request.as_bytes());
        bytes[42..58].copy_from_slice(f.case.as_uuid().as_bytes());
        bytes[58..74].copy_from_slice(f.document.id.as_uuid().as_bytes());
        bytes[74..78].copy_from_slice(&f.document.version.get().to_be_bytes());
        bytes[78..110].copy_from_slice(f.content_digest.as_bytes());
        bytes[110..126].copy_from_slice(f.client.as_uuid().as_bytes());
        bytes[126..142].copy_from_slice(f.firm_signer.as_uuid().as_bytes());
        bytes[142..158].copy_from_slice(f.requester.as_uuid().as_bytes());
        bytes[158..190].copy_from_slice(f.client_leaf.as_bytes());
        bytes[190..222].copy_from_slice(f.firm_leaf.as_bytes());
        bytes[222..230].copy_from_slice(&f.created_at_seconds.to_be_bytes());
        bytes[230..238].copy_from_slice(&f.expires_at_seconds.to_be_bytes());
        bytes[238] = PARTY_COUNT;
        bytes[239] = COMPLETION_RULE;
        bytes
    }

    /// Decodes one exact request, reapplying every constructor rule.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ContractStatementError> {
        if bytes.len() != REQUEST_LEN || &bytes[..8] != REQUEST_MAGIC || bytes[8..10] != [1, 1] {
            return Err(ContractStatementError::StatementMismatch);
        }
        let identities = [10, 26, 42, 58, 110, 126, 142].map(|offset| uuid(bytes, offset));
        if identities.iter().any(Uuid::is_nil) {
            return Err(ContractStatementError::InvalidIdentity);
        }
        let version = DocumentVersion::new(u32::from_be_bytes(field(bytes, 74)?))
            .map_err(|_| ContractStatementError::StatementMismatch)?;
        if bytes[238] != PARTY_COUNT || bytes[239] != COMPLETION_RULE {
            return Err(ContractStatementError::StatementMismatch);
        }
        let request = Self::new(ContractRequestFields {
            deployment: identities[0],
            request: identities[1],
            case: CaseId::from_uuid(identities[2]),
            document: DocumentVersionRef {
                id: DocumentId::from_uuid(identities[3]),
                version,
            },
            content_digest: Sha256Digest::from_array(field(bytes, 78)?),
            client: UserId::from_uuid(identities[4]),
            firm_signer: UserId::from_uuid(identities[5]),
            requester: UserId::from_uuid(identities[6]),
            client_leaf: Sha256Digest::from_array(field(bytes, 158)?),
            firm_leaf: Sha256Digest::from_array(field(bytes, 190)?),
            created_at_seconds: i64::from_be_bytes(field(bytes, 222)?),
            expires_at_seconds: i64::from_be_bytes(field(bytes, 230)?),
        })?;
        request.require_exact_bytes(bytes)?;
        Ok(request)
    }

    /// Rejects any difference from the reconstructed request.
    pub fn require_exact_bytes(&self, supplied: &[u8]) -> Result<(), ContractStatementError> {
        require_exact(supplied, &self.canonical_bytes())
    }
}

/// Reads one UUID; callers have already checked the full length.
fn uuid(bytes: &[u8], offset: usize) -> Uuid {
    let mut value = [0; 16];
    value.copy_from_slice(&bytes[offset..offset + 16]);
    Uuid::from_bytes(value)
}

fn require_exact(supplied: &[u8], expected: &[u8]) -> Result<(), ContractStatementError> {
    if supplied != expected {
        return Err(ContractStatementError::StatementMismatch);
    }
    Ok(())
}
