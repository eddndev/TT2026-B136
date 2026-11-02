//! The evidence manifest of bilateral contract signing: its model, its RFC
//! 8785 encoder and its strict parser.
//!
//! The manifest has `profile_version` 2 and purpose
//! `bilateral-contract-signing`. It lists every covered entry of a full
//! contract package with its party (0 for the shared entries, 1 for the
//! Client, 2 for the firm signer), role, SHA-256 and size, and both signers
//! in position order. Each party and role names exactly one fixed entry, from
//! [`ContractRole::entry`], so the server export and the validator share the
//! entry names. Only the canonical bytes are admitted. The individual
//! signing manifest and its parser are a separate profile and are not
//! touched. Rationale: the section "Evidence packages" of
//! `docs/adr/0086-bilateral-contract-signing.md`.

mod entries;
mod schema;
#[cfg(test)]
mod tests;

use super::{json_finding, ALGORITHMS, PROFILE};
use crate::certificate;
use crate::json::{self, Limits, Value};
use crate::report::{hex, Finding};

pub use entries::{
    PartyEntries, DOCUMENT_ENTRY, DOCUMENT_TOKEN_ENTRY, PARTY_ENTRIES, REQUEST_ENTRY, ROOT_ENTRY,
    STATEMENT_TOKEN_ENTRY, STATEMENT_TSA_CHAIN_ENTRY, TSA_CHAIN_ENTRY,
};

/// Value of the `profile_version` member.
pub const PROFILE_VERSION: u64 = 2;
/// Value of the `purpose` member.
pub const PURPOSE: &str = "bilateral-contract-signing";

/// Package entry holding the manifest.
pub const ENTRY: &str = "contrato-manifiesto.json";
/// Package entry holding the time-stamp response Tf over the manifest digest.
pub const TOKEN_ENTRY: &str = "contrato-manifiesto.json.tsr";

/// Most JSON values a manifest may hold: 23 artifacts and two signers need
/// about two hundred.
const MAX_VALUES: usize = 512;

/// What one covered entry is in a contract package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractRole {
    Document,
    DocumentTimestamp,
    Request,
    IssuerCertificate,
    TsaChain,
    Statement,
    StatementSignature,
    SignerCertificate,
    IssuerCrl,
    BindingStatement,
    BindingSignature,
    BindingRoot,
    BindingCrl,
    StatementTimestamp,
    StatementTsaChain,
}

impl ContractRole {
    pub const ALL: [ContractRole; 15] = [
        ContractRole::Document,
        ContractRole::DocumentTimestamp,
        ContractRole::Request,
        ContractRole::IssuerCertificate,
        ContractRole::TsaChain,
        ContractRole::Statement,
        ContractRole::StatementSignature,
        ContractRole::SignerCertificate,
        ContractRole::IssuerCrl,
        ContractRole::BindingStatement,
        ContractRole::BindingSignature,
        ContractRole::BindingRoot,
        ContractRole::BindingCrl,
        ContractRole::StatementTimestamp,
        ContractRole::StatementTsaChain,
    ];

    /// The name the manifest writes for this role.
    pub fn as_str(self) -> &'static str {
        match self {
            ContractRole::Document => "document",
            ContractRole::DocumentTimestamp => "document-timestamp",
            ContractRole::Request => "request",
            ContractRole::IssuerCertificate => "issuer-certificate",
            ContractRole::TsaChain => "tsa-chain",
            ContractRole::Statement => "statement",
            ContractRole::StatementSignature => "statement-signature",
            ContractRole::SignerCertificate => "signer-certificate",
            ContractRole::IssuerCrl => "issuer-crl",
            ContractRole::BindingStatement => "binding-statement",
            ContractRole::BindingSignature => "binding-signature",
            ContractRole::BindingRoot => "binding-root",
            ContractRole::BindingCrl => "binding-crl",
            ContractRole::StatementTimestamp => "statement-timestamp",
            ContractRole::StatementTsaChain => "statement-tsa-chain",
        }
    }

    /// The role written as `name`, if any.
    pub fn parse(name: &str) -> Option<ContractRole> {
        ContractRole::ALL
            .into_iter()
            .find(|role| role.as_str() == name)
    }

    /// The entry that holds this role for `party`, or `None` when the role
    /// does not belong to that party.
    pub fn entry(self, party: u8) -> Option<&'static str> {
        let shared = match self {
            ContractRole::Document => Some(DOCUMENT_ENTRY),
            ContractRole::DocumentTimestamp => Some(DOCUMENT_TOKEN_ENTRY),
            ContractRole::Request => Some(REQUEST_ENTRY),
            ContractRole::IssuerCertificate => Some(ROOT_ENTRY),
            ContractRole::TsaChain => Some(TSA_CHAIN_ENTRY),
            _ => None,
        };
        if let Some(entry) = shared {
            return (party == 0).then_some(entry);
        }
        match (self, party) {
            (ContractRole::StatementTimestamp, 1) => return Some(STATEMENT_TOKEN_ENTRY),
            (ContractRole::StatementTsaChain, 1) => return Some(STATEMENT_TSA_CHAIN_ENTRY),
            (ContractRole::StatementTimestamp | ContractRole::StatementTsaChain, _) => return None,
            _ => {}
        }
        let entries = PARTY_ENTRIES.get(usize::from(party).checked_sub(1)?)?;
        entries
            .members()
            .into_iter()
            .find(|(_, role, _)| *role == self)
            .map(|(_, _, entry)| entry)
    }
}

/// The application role of a signer account; it is not part of any signed
/// bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRole {
    Client,
    Owner,
    Litigator,
}

impl AccountRole {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountRole::Client => "client",
            AccountRole::Owner => "owner",
            AccountRole::Litigator => "litigator",
        }
    }
}

/// One covered entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractArtifact {
    pub entry: String,
    /// 0 for a shared entry, otherwise the position of its party.
    pub party: u8,
    pub role: ContractRole,
    pub sha256: [u8; 32],
    pub size: u64,
}

/// One party; its `id`, `position`, `side` and `entries` follow from its
/// index in [`ContractManifest::signers`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractSigner {
    pub account_id: String,
    pub account_role: AccountRole,
    pub binding_id: String,
    /// Unix seconds, `0..=2^53 - 1`.
    pub binding_registered_at: i64,
    pub operation_id: String,
}

/// The parsed manifest; identifiers are lowercase hyphenated UUIDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractManifest {
    pub deployment_id: String,
    pub case_id: String,
    pub request_id: String,
    pub document_id: String,
    pub document_version: u64,
    pub document_sha256: [u8; 32],
    /// SHA-256 of the 240 request bytes.
    pub request_sha256: [u8; 32],
    /// Unix seconds, the request's `expires_at`.
    pub expires_at: i64,
    /// Sorted by entry name in byte order when parsed; [`encode`] sorts.
    pub artifacts: Vec<ContractArtifact>,
    /// The Client at position 1, then the firm signer at position 2.
    pub signers: [ContractSigner; 2],
}

impl ContractManifest {
    /// The artifact row of `role` for `party` holding `bytes`; the entry is
    /// empty when the role does not belong to that party, which [`encode`]
    /// then refuses.
    pub fn artifact(party: u8, role: ContractRole, bytes: &[u8]) -> ContractArtifact {
        ContractArtifact {
            entry: role.entry(party).unwrap_or_default().to_owned(),
            party,
            role,
            sha256: certificate::sha256(bytes),
            size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        }
    }
}

/// The `id` of the signer at `index`, 0 or 1.
fn signer_id(index: usize) -> &'static str {
    ["signer-1", "signer-2"][index]
}

/// The `side` of the signer at `index`, 0 or 1.
fn side(index: usize) -> &'static str {
    ["client", "firm"][index]
}

/// The canonical bytes of `manifest`, with the artifacts sorted by entry
/// name in byte order.
///
/// Fails with the finding [`parse`] would report when the result would not
/// be admitted, so the server never writes a manifest the validator rejects.
pub fn encode(manifest: &ContractManifest) -> Result<Vec<u8>, Finding> {
    let mut artifacts: Vec<&ContractArtifact> = manifest.artifacts.iter().collect();
    artifacts.sort_by(|left, right| left.entry.as_bytes().cmp(right.entry.as_bytes()));
    let text = |value: &str| Value::String(value.to_owned());
    let integer = |value: i128| Value::Integer(value);
    let member = |key: &str, value: Value| (key.to_owned(), value);
    let artifacts = artifacts
        .into_iter()
        .map(|artifact| {
            Value::Object(vec![
                member("entry", text(&artifact.entry)),
                member("party", integer(i128::from(artifact.party))),
                member("role", text(artifact.role.as_str())),
                member("sha256", text(&hex(&artifact.sha256))),
                member("size", integer(i128::from(artifact.size))),
            ])
        })
        .collect();
    let signers = manifest
        .signers
        .iter()
        .enumerate()
        .map(|(index, signer)| {
            let entries = PARTY_ENTRIES[index]
                .members()
                .iter()
                .map(|(key, _, entry)| member(key, text(entry)))
                .collect();
            Value::Object(vec![
                member("id", text(signer_id(index))),
                member("position", integer(index as i128 + 1)),
                member("side", text(side(index))),
                member("account_id", text(&signer.account_id)),
                member("account_role", text(signer.account_role.as_str())),
                member("binding_id", text(&signer.binding_id)),
                member(
                    "binding_registered_at",
                    integer(i128::from(signer.binding_registered_at)),
                ),
                member("operation_id", text(&signer.operation_id)),
                member("entries", Value::Object(entries)),
            ])
        })
        .collect();
    let algorithms = ALGORITHMS
        .iter()
        .map(|(key, value)| member(key, text(value)))
        .collect();
    let value = Value::Object(vec![
        member("profile", text(PROFILE)),
        member("profile_version", integer(i128::from(PROFILE_VERSION))),
        member("purpose", text(PURPOSE)),
        member("deployment_id", text(&manifest.deployment_id)),
        member("case_id", text(&manifest.case_id)),
        member("request_id", text(&manifest.request_id)),
        member("document_id", text(&manifest.document_id)),
        member(
            "document_version",
            integer(i128::from(manifest.document_version)),
        ),
        member("document_sha256", text(&hex(&manifest.document_sha256))),
        member("request_sha256", text(&hex(&manifest.request_sha256))),
        member("expires_at", integer(i128::from(manifest.expires_at))),
        member("algorithms", Value::Object(algorithms)),
        member("artifacts", Value::Array(artifacts)),
        member("signers", Value::Array(signers)),
    ]);
    let bytes = json::canonical(&value).map_err(json_finding)?;
    parse(&bytes)?;
    Ok(bytes)
}

/// Parse the manifest entry `bytes`.
///
/// Bytes that are not the canonical serialization of their value fail with
/// the JSON code; artifacts out of byte order or repeated fail with their
/// own codes; any other departure from the exact schema fails with
/// `manifest.contract_schema`.
pub fn parse(bytes: &[u8]) -> Result<ContractManifest, Finding> {
    let value = json::parse_canonical(
        bytes,
        Limits {
            max_values: MAX_VALUES,
        },
    )
    .map_err(json_finding)?;
    schema::manifest(&value)
}
