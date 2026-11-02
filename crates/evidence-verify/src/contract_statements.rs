//! Request and party statements of bilateral contract signing.
//!
//! The byte layouts and decoding rules reproduce
//! `crates/domain/src/document_signing/contract.rs` and
//! `crates/domain/src/document_signing/contract/party.rs`; both copies are
//! tested against `crates/domain/tests/fixtures/contract_statement_vectors.json`.
//! Request rejections emit `contract.request_schema` and party statement
//! rejections emit the statement codes of the shared prefix. Details name the
//! offending field and never echo submitted identifiers or digests.
//! Rationale: docs/adr/0086-bilateral-contract-signing.md.

use crate::report::Finding;
use crate::statements::{
    exact, field, header, identity, prefix, write_prefix, BindingStatement, FORMAT,
};

/// Exact length of a `CTRTREQ1` request.
pub const REQUEST_LEN: usize = 240;
/// Exact length of a `CTRTSIG1` party statement.
pub const PARTY_STATEMENT_LEN: usize = 303;

const REQUEST_MAGIC: &[u8; 8] = b"CTRTREQ1";
const PARTY_MAGIC: &[u8; 8] = b"CTRTSIG1";
const PARTY_COUNT: u8 = 2;
/// Every party signs before `expires_at` according to its time-stamp.
const COMPLETION_RULE: u8 = 1;
const REQUEST_SCHEMA: &str = "contract.request_schema";

/// The signature request the server builds; nobody signs it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractRequest {
    pub deployment: [u8; 16],
    pub request: [u8; 16],
    pub case: [u8; 16],
    pub document: [u8; 16],
    pub document_version: u32,
    pub document_sha256: [u8; 32],
    pub client_account: [u8; 16],
    pub firm_account: [u8; 16],
    pub requester_account: [u8; 16],
    pub client_certificate_sha256: [u8; 32],
    pub firm_certificate_sha256: [u8; 32],
    /// Unix seconds.
    pub created_at: i64,
    /// Unix seconds.
    pub expires_at: i64,
}

/// One party's acceptance of an exact request. Position 1 is the Client and
/// carries zero chain digests; position 2 is the firm signer and carries the
/// digests of the position 1 signature and its time-stamp response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyStatement {
    pub deployment: [u8; 16],
    pub root_sha256: [u8; 32],
    pub trust_revision: u32,
    pub account: [u8; 16],
    pub account_revision: u64,
    pub authentication_generation: u64,
    pub binding: [u8; 16],
    pub certificate_sha256: [u8; 32],
    pub operation: [u8; 16],
    pub request: [u8; 16],
    pub request_sha256: [u8; 32],
    pub document_sha256: [u8; 32],
    pub position: u8,
    pub prior_signature_sha256: [u8; 32],
    pub prior_token_sha256: [u8; 32],
}

/// Decodes one exact request, applying every rule of the domain decoder; it
/// recomputes no digest and proves no eligibility.
pub fn decode_request(bytes: &[u8]) -> Result<ContractRequest, Finding> {
    let schema = |detail: &'static str| Finding::failed(REQUEST_SCHEMA, detail);
    if bytes.len() != REQUEST_LEN {
        return Err(schema("length"));
    }
    if &bytes[..8] != REQUEST_MAGIC {
        return Err(schema("purpose"));
    }
    if bytes[8..10] != FORMAT {
        return Err(schema("format"));
    }
    let mut ids = [[0; 16]; 7];
    for (slot, (offset, name)) in ids.iter_mut().zip([
        (10, "deployment"),
        (26, "request"),
        (42, "case"),
        (58, "document"),
        (110, "client_account"),
        (126, "firm_account"),
        (142, "requester_account"),
    ]) {
        *slot = field(bytes, offset);
        if *slot == [0; 16] {
            return Err(schema(name));
        }
    }
    let [deployment, request, case, document, client_account, firm_account, requester_account] =
        ids;
    let document_version = u32::from_be_bytes(field(bytes, 74));
    if document_version == 0 {
        return Err(schema("document_version"));
    }
    if bytes[238] != PARTY_COUNT {
        return Err(schema("parties"));
    }
    if bytes[239] != COMPLETION_RULE {
        return Err(schema("completion_rule"));
    }
    if client_account == firm_account || client_account == requester_account {
        return Err(schema("same_party"));
    }
    let client_certificate_sha256 = field(bytes, 158);
    let firm_certificate_sha256 = field(bytes, 190);
    if client_certificate_sha256 == firm_certificate_sha256 {
        return Err(schema("same_certificate"));
    }
    let created_at = i64::from_be_bytes(field(bytes, 222));
    let expires_at = i64::from_be_bytes(field(bytes, 230));
    if created_at >= expires_at {
        return Err(schema("validity"));
    }
    Ok(ContractRequest {
        deployment,
        request,
        case,
        document,
        document_version,
        document_sha256: field(bytes, 78),
        client_account,
        firm_account,
        requester_account,
        client_certificate_sha256,
        firm_certificate_sha256,
        created_at,
        expires_at,
    })
}

/// Writes the canonical request bytes without validating the values.
pub fn encode_request(request: &ContractRequest) -> [u8; REQUEST_LEN] {
    let mut bytes = [0; REQUEST_LEN];
    bytes[..8].copy_from_slice(REQUEST_MAGIC);
    bytes[8..10].copy_from_slice(&FORMAT);
    bytes[10..26].copy_from_slice(&request.deployment);
    bytes[26..42].copy_from_slice(&request.request);
    bytes[42..58].copy_from_slice(&request.case);
    bytes[58..74].copy_from_slice(&request.document);
    bytes[74..78].copy_from_slice(&request.document_version.to_be_bytes());
    bytes[78..110].copy_from_slice(&request.document_sha256);
    bytes[110..126].copy_from_slice(&request.client_account);
    bytes[126..142].copy_from_slice(&request.firm_account);
    bytes[142..158].copy_from_slice(&request.requester_account);
    bytes[158..190].copy_from_slice(&request.client_certificate_sha256);
    bytes[190..222].copy_from_slice(&request.firm_certificate_sha256);
    bytes[222..230].copy_from_slice(&request.created_at.to_be_bytes());
    bytes[230..238].copy_from_slice(&request.expires_at.to_be_bytes());
    bytes[238] = PARTY_COUNT;
    bytes[239] = COMPLETION_RULE;
    bytes
}

/// Decodes one exact party statement, applying every rule of the domain
/// decoder; it recomputes no digest and checks no signature.
pub fn decode_party_statement(bytes: &[u8]) -> Result<PartyStatement, Finding> {
    let head = header(bytes, PARTY_MAGIC, PARTY_STATEMENT_LEN).and_then(prefix)?;
    let position = bytes[238];
    let prior_signature_sha256 = field(bytes, 239);
    let prior_token_sha256 = field(bytes, 271);
    let invalid_position = || Finding::failed("statement.position", "position");
    if !matches!(position, 1 | 2) {
        return Err(invalid_position());
    }
    let operation = identity(bytes, 142, "operation")?;
    let request = identity(bytes, 158, "request")?;
    let empty_chain = prior_signature_sha256 == [0; 32] || prior_token_sha256 == [0; 32];
    if position == 2 && empty_chain {
        return Err(invalid_position());
    }
    let statement = PartyStatement {
        deployment: head.deployment,
        root_sha256: head.root_sha256,
        trust_revision: head.trust_revision,
        account: head.account,
        account_revision: head.account_revision,
        authentication_generation: head.authentication_generation,
        binding: head.binding,
        certificate_sha256: head.certificate_sha256,
        operation,
        request,
        request_sha256: field(bytes, 174),
        document_sha256: field(bytes, 206),
        position,
        prior_signature_sha256: if position == 1 {
            [0; 32]
        } else {
            prior_signature_sha256
        },
        prior_token_sha256: if position == 1 {
            [0; 32]
        } else {
            prior_token_sha256
        },
    };
    exact(bytes, &encode_party_statement(&statement))?;
    Ok(statement)
}

/// Writes the canonical party statement bytes without validating the values.
pub fn encode_party_statement(statement: &PartyStatement) -> [u8; PARTY_STATEMENT_LEN] {
    let mut bytes = [0; PARTY_STATEMENT_LEN];
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
    write_prefix(&mut bytes, PARTY_MAGIC, &head);
    bytes[142..158].copy_from_slice(&statement.operation);
    bytes[158..174].copy_from_slice(&statement.request);
    bytes[174..206].copy_from_slice(&statement.request_sha256);
    bytes[206..238].copy_from_slice(&statement.document_sha256);
    bytes[238] = statement.position;
    bytes[239..271].copy_from_slice(&statement.prior_signature_sha256);
    bytes[271..303].copy_from_slice(&statement.prior_token_sha256);
    bytes
}

#[cfg(test)]
mod tests;
