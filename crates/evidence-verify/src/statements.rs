//! Canonical binding and consent statements of individual signing.
//!
//! The byte layouts reproduce `crates/domain/src/document_signing.rs` and
//! `crates/domain/src/document_signing/consent.rs`; the decoding rules
//! reproduce `crates/domain/src/document_signing/decode.rs`. Every rejection
//! is a failed finding, because the server never writes such bytes. Details
//! name the offending field and never echo submitted identifiers or digests.
//! Rationale: docs/adr/0085-in-browser-evidence-validation.md.

use crate::report::Finding;

/// Exact length of a `DOCBIND1` binding statement.
pub const BINDING_LEN: usize = 142;
/// Exact length of a `DOCSIGN1` consent statement.
pub const CONSENT_LEN: usize = 258;

/// Registration of the holder's own signing certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingStatement {
    pub deployment: [u8; 16],
    pub root_sha256: [u8; 32],
    pub trust_revision: u32,
    pub account: [u8; 16],
    pub account_revision: u64,
    pub authentication_generation: u64,
    pub binding: [u8; 16],
    pub certificate_sha256: [u8; 32],
}

/// Consent of the holder to one exact document version and detached signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentStatement {
    pub deployment: [u8; 16],
    pub root_sha256: [u8; 32],
    pub trust_revision: u32,
    pub account: [u8; 16],
    pub account_revision: u64,
    pub authentication_generation: u64,
    pub binding: [u8; 16],
    pub certificate_sha256: [u8; 32],
    pub operation: [u8; 16],
    pub case: [u8; 16],
    pub document: [u8; 16],
    pub document_version: u32,
    pub document_sha256: [u8; 32],
    pub signature_sha256: [u8; 32],
}

/// Decodes one exact binding statement, applying every rule of the domain
/// decoder; it checks no signature and proves no identity.
pub fn decode_binding(bytes: &[u8]) -> Result<BindingStatement, Finding> {
    let statement = header(bytes, BINDING_MAGIC, BINDING_LEN).and_then(prefix)?;
    exact(bytes, &encode_binding(&statement))?;
    Ok(statement)
}

/// Decodes one exact consent statement, applying every rule of the domain
/// decoder; it recomputes no digest and checks no signature.
pub fn decode_consent(bytes: &[u8]) -> Result<ConsentStatement, Finding> {
    let head = header(bytes, CONSENT_MAGIC, CONSENT_LEN).and_then(prefix)?;
    let document_version = u32::from_be_bytes(field(bytes, 190));
    if document_version == 0 {
        return Err(Finding::failed(
            "statement.document_version",
            "document_version",
        ));
    }
    let operation = identity(bytes, 142, "operation")?;
    let case = identity(bytes, 158, "case")?;
    let document = identity(bytes, 174, "document")?;
    let statement = ConsentStatement {
        deployment: head.deployment,
        root_sha256: head.root_sha256,
        trust_revision: head.trust_revision,
        account: head.account,
        account_revision: head.account_revision,
        authentication_generation: head.authentication_generation,
        binding: head.binding,
        certificate_sha256: head.certificate_sha256,
        operation,
        case,
        document,
        document_version,
        document_sha256: field(bytes, 194),
        signature_sha256: field(bytes, 226),
    };
    exact(bytes, &encode_consent(&statement))?;
    Ok(statement)
}

/// Writes the canonical binding bytes without validating the values.
pub fn encode_binding(statement: &BindingStatement) -> [u8; BINDING_LEN] {
    let mut bytes = [0; BINDING_LEN];
    write_prefix(&mut bytes, BINDING_MAGIC, statement);
    bytes
}

/// Writes the canonical consent bytes without validating the values.
pub fn encode_consent(statement: &ConsentStatement) -> [u8; CONSENT_LEN] {
    let mut bytes = [0; CONSENT_LEN];
    let head = BindingStatement {
        deployment: statement.deployment,
        root_sha256: statement.root_sha256,
        trust_revision: statement.trust_revision,
        account: statement.account,
        account_revision: statement.account_revision,
        authentication_generation: statement.authentication_generation,
        binding: statement.binding,
        certificate_sha256: statement.certificate_sha256,
    };
    write_prefix(&mut bytes, CONSENT_MAGIC, &head);
    bytes[142..158].copy_from_slice(&statement.operation);
    bytes[158..174].copy_from_slice(&statement.case);
    bytes[174..190].copy_from_slice(&statement.document);
    bytes[190..194].copy_from_slice(&statement.document_version.to_be_bytes());
    bytes[194..226].copy_from_slice(&statement.document_sha256);
    bytes[226..258].copy_from_slice(&statement.signature_sha256);
    bytes
}

/// Formats a UUID as lowercase hexadecimal in the 8-4-4-4-12 grouping.
pub fn uuid_text(uuid: [u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(36);
    for (index, byte) in uuid.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    text
}

const BINDING_MAGIC: &[u8; 8] = b"DOCBIND1";
const CONSENT_MAGIC: &[u8; 8] = b"DOCSIGN1";
/// Byte 8 is the purpose number and byte 9 the format version; both are 1.
const FORMAT: [u8; 2] = [1, 1];

fn header<'a>(bytes: &'a [u8], magic: &[u8; 8], length: usize) -> Result<&'a [u8], Finding> {
    if bytes.len() != length {
        return Err(Finding::failed(
            "statement.length",
            format!("{} bytes", bytes.len()),
        ));
    }
    if &bytes[..8] != magic {
        return Err(Finding::failed("statement.purpose", "magic"));
    }
    if bytes[8..10] != FORMAT {
        return Err(Finding::failed(
            "statement.format",
            "purpose and format version",
        ));
    }
    Ok(bytes)
}

/// Reads the shared prefix in the order of the domain constructors: account,
/// account counters, then deployment, binding and trust revision.
fn prefix(bytes: &[u8]) -> Result<BindingStatement, Finding> {
    let account = identity(bytes, 62, "account")?;
    let account_revision = u64::from_be_bytes(field(bytes, 78));
    let authentication_generation = u64::from_be_bytes(field(bytes, 86));
    if account_revision > i64::MAX as u64 {
        return Err(Finding::failed(
            "statement.account_version",
            "account_revision",
        ));
    }
    if authentication_generation > account_revision {
        return Err(Finding::failed(
            "statement.account_version",
            "authentication_generation",
        ));
    }
    let deployment = identity(bytes, 10, "deployment")?;
    let binding = identity(bytes, 94, "binding")?;
    let trust_revision = u32::from_be_bytes(field(bytes, 58));
    if trust_revision == 0 {
        return Err(Finding::failed(
            "statement.trust_revision",
            "trust_revision",
        ));
    }
    Ok(BindingStatement {
        deployment,
        root_sha256: field(bytes, 26),
        trust_revision,
        account,
        account_revision,
        authentication_generation,
        binding,
        certificate_sha256: field(bytes, 110),
    })
}

fn write_prefix(bytes: &mut [u8], magic: &[u8; 8], statement: &BindingStatement) {
    bytes[..8].copy_from_slice(magic);
    bytes[8..10].copy_from_slice(&FORMAT);
    bytes[10..26].copy_from_slice(&statement.deployment);
    bytes[26..58].copy_from_slice(&statement.root_sha256);
    bytes[58..62].copy_from_slice(&statement.trust_revision.to_be_bytes());
    bytes[62..78].copy_from_slice(&statement.account);
    bytes[78..86].copy_from_slice(&statement.account_revision.to_be_bytes());
    bytes[86..94].copy_from_slice(&statement.authentication_generation.to_be_bytes());
    bytes[94..110].copy_from_slice(&statement.binding);
    bytes[110..142].copy_from_slice(&statement.certificate_sha256);
}

/// Copies a fixed-width field; callers have already checked the full length.
fn field<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    let mut value = [0; N];
    value.copy_from_slice(&bytes[offset..offset + N]);
    value
}

fn identity(bytes: &[u8], offset: usize, name: &'static str) -> Result<[u8; 16], Finding> {
    let value = field(bytes, offset);
    if value == [0; 16] {
        return Err(Finding::failed("statement.nil_identity", name));
    }
    Ok(value)
}

/// Re-encodes the decoded values and requires the supplied bytes exactly, as
/// the domain decoder does after construction.
fn exact(supplied: &[u8], expected: &[u8]) -> Result<(), Finding> {
    if supplied != expected {
        return Err(Finding::failed(
            "statement.not_canonical",
            "re-encoded bytes differ",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
