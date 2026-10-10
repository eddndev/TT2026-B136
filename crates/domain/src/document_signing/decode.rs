//! Canonical public statement decoding through the original invariant constructors.

use super::{
    DocumentConsent, SignerAccount, SigningBindingMaterial, SigningBindingStatement, SigningError,
};
use crate::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    identity::{Role, UserId},
};
use uuid::Uuid;

impl SigningBindingStatement {
    /// Decodes one exact registration without claiming identity or signature validity.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        let (account, material) = prefix(bytes, b"DOCBIND1", 142)?;
        let statement = Self::new(account, account.id(), material)?;
        statement.require_exact_bytes(bytes)?;
        Ok(statement)
    }
}

impl DocumentConsent {
    /// Decodes one exact consent, reapplying every public reference invariant.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        let (account, material) = prefix(bytes, b"DOCSIGN1", 258)?;
        let document = DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_bytes(field(bytes, 174)?)),
            version: DocumentVersion::new(u32::from_be_bytes(field(bytes, 190)?))
                .map_err(|_| SigningError::StatementMismatch)?,
        };
        let statement = Self::new(
            account,
            material,
            Uuid::from_bytes(field(bytes, 142)?),
            CaseId::from_uuid(Uuid::from_bytes(field(bytes, 158)?)),
            document,
            Sha256Digest::from_array(field(bytes, 194)?),
            Sha256Digest::from_array(field(bytes, 226)?),
        )?;
        statement.require_exact_bytes(bytes)?;
        Ok(statement)
    }
}

fn prefix(
    bytes: &[u8],
    purpose: &[u8; 8],
    length: usize,
) -> Result<(SignerAccount, SigningBindingMaterial), SigningError> {
    if bytes.len() != length || &bytes[..8] != purpose || bytes[8..10] != [1, 1] {
        return Err(SigningError::StatementMismatch);
    }
    let account = SignerAccount::new(
        UserId::from_uuid(Uuid::from_bytes(field(bytes, 62)?)),
        Role::Litigator,
        true,
        u64::from_be_bytes(field(bytes, 78)?),
        u64::from_be_bytes(field(bytes, 86)?),
    )?;
    let material = SigningBindingMaterial::new(
        Uuid::from_bytes(field(bytes, 10)?),
        Uuid::from_bytes(field(bytes, 94)?),
        Sha256Digest::from_array(field(bytes, 26)?),
        Sha256Digest::from_array(field(bytes, 110)?),
        u32::from_be_bytes(field(bytes, 58)?),
    )?;
    Ok((account, material))
}

fn field<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], SigningError> {
    bytes
        .get(offset..offset + N)
        .and_then(|value| value.try_into().ok())
        .ok_or(SigningError::StatementMismatch)
}
