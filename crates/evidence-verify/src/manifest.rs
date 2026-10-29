//! The evidence manifest of individual signing v2: its model, its RFC 8785
//! encoder and its strict parser.
//!
//! The manifest lists every covered entry of the package with its role,
//! SHA-256 and size, and the holder who signed, and a time-stamp token
//! covers its digest. The server builds it with [`encode`] inside the
//! acceptance operation and the validator reads it with [`parse`], so both
//! use the same field names, role names and entry names, the latter from
//! [`role_entry`]. Only the canonical bytes are admitted: the parsed value
//! must serialize back to exactly the entry bytes. Rationale: the section
//! "The evidence manifest" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

pub(crate) mod schema;
#[cfg(test)]
mod tests;

use crate::certificate;
use crate::json::{self, JsonError, Limits, Value};
use crate::report::{hex, Finding};

/// Value of the `profile` member.
pub const PROFILE: &str = "qadra-evidence-manifest";
/// Value of the `profile_version` member.
pub const PROFILE_VERSION: u64 = 1;
/// Value of the `purpose` member.
pub const PURPOSE: &str = "individual-document-signing";
/// Members of `algorithms` and their only admitted values.
pub const ALGORITHMS: [(&str, &str); 3] = [
    ("digest", "sha-256"),
    ("signature", "rsa-pkcs1v15-sha256"),
    ("timestamp", "rfc3161-sha256"),
];
/// The `id` of the only signer this profile admits.
pub const SIGNER_ID: &str = "signer-1";

/// Package entry holding the manifest.
pub const ENTRY: &str = "manifiesto.json";
/// Package entry holding the time-stamp response over the manifest digest.
pub const TOKEN_ENTRY: &str = "manifiesto.json.tsr";
/// Package entry holding the CRL of the time-stamping certificate issuer.
pub const TSA_CRL_ENTRY: &str = "tsa-crl.pem";

/// Most JSON values a manifest may hold: one signer and one artifact per
/// role need about a hundred.
const MAX_VALUES: usize = 512;

/// What one covered entry is in the package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Document,
    DocumentSignature,
    DocumentTimestamp,
    SignerCertificate,
    IssuerCertificate,
    IssuerCrl,
    TsaChain,
    TsaIssuerCrl,
    BindingStatement,
    BindingSignature,
    BindingRoot,
    BindingCrl,
    ConsentStatement,
    ConsentSignature,
}

impl Role {
    /// Every role; each names exactly one entry of a package.
    pub const ALL: [Role; 14] = [
        Role::Document,
        Role::DocumentSignature,
        Role::DocumentTimestamp,
        Role::SignerCertificate,
        Role::IssuerCertificate,
        Role::IssuerCrl,
        Role::TsaChain,
        Role::TsaIssuerCrl,
        Role::BindingStatement,
        Role::BindingSignature,
        Role::BindingRoot,
        Role::BindingCrl,
        Role::ConsentStatement,
        Role::ConsentSignature,
    ];

    /// The name the manifest writes for this role.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Document => "document",
            Role::DocumentSignature => "document-signature",
            Role::DocumentTimestamp => "document-timestamp",
            Role::SignerCertificate => "signer-certificate",
            Role::IssuerCertificate => "issuer-certificate",
            Role::IssuerCrl => "issuer-crl",
            Role::TsaChain => "tsa-chain",
            Role::TsaIssuerCrl => "tsa-issuer-crl",
            Role::BindingStatement => "binding-statement",
            Role::BindingSignature => "binding-signature",
            Role::BindingRoot => "binding-root",
            Role::BindingCrl => "binding-crl",
            Role::ConsentStatement => "consent-statement",
            Role::ConsentSignature => "consent-signature",
        }
    }

    /// The role written as `name`, if any.
    pub fn parse(name: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.as_str() == name)
    }
}

/// The package entry that holds `role` when the document is named
/// `document`; the single source of entry names for the server export and
/// the validator.
pub fn role_entry(role: Role, document: &str) -> String {
    let fixed = match role {
        Role::Document => return document.to_owned(),
        Role::DocumentSignature => return format!("{document}.sig"),
        Role::DocumentTimestamp => return format!("{document}.tsr"),
        Role::SignerCertificate => "certificado.pem",
        Role::IssuerCertificate => "ca.pem",
        Role::IssuerCrl => "crl.pem",
        Role::TsaChain => "tsa-chain.pem",
        Role::TsaIssuerCrl => TSA_CRL_ENTRY,
        Role::BindingStatement => "certificate-binding.bin",
        Role::BindingSignature => "certificate-binding.sig",
        Role::BindingRoot => "binding-root.der",
        Role::BindingCrl => "binding-crl.der",
        Role::ConsentStatement => "document-consent.bin",
        Role::ConsentSignature => "document-consent.sig",
    };
    fixed.to_owned()
}

/// One covered entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub entry: String,
    pub role: Role,
    pub sha256: [u8; 32],
    pub size: u64,
}

/// The entries one signer's objects are in, by member of `entries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerEntries {
    pub document_signature: String,
    pub signer_certificate: String,
    pub binding_statement: String,
    pub binding_signature: String,
    pub binding_root: String,
    pub binding_crl: String,
    pub consent_statement: String,
    pub consent_signature: String,
}

impl SignerEntries {
    /// The entries of a signer of the document `document`, by [`role_entry`].
    pub fn for_document(document: &str) -> Self {
        let entry = |role| role_entry(role, document);
        Self {
            document_signature: entry(Role::DocumentSignature),
            signer_certificate: entry(Role::SignerCertificate),
            binding_statement: entry(Role::BindingStatement),
            binding_signature: entry(Role::BindingSignature),
            binding_root: entry(Role::BindingRoot),
            binding_crl: entry(Role::BindingCrl),
            consent_statement: entry(Role::ConsentStatement),
            consent_signature: entry(Role::ConsentSignature),
        }
    }

    /// Each member name, the role its entry must have and the entry.
    pub fn members(&self) -> [(&'static str, Role, &str); 8] {
        [
            (
                "document_signature",
                Role::DocumentSignature,
                &self.document_signature,
            ),
            (
                "signer_certificate",
                Role::SignerCertificate,
                &self.signer_certificate,
            ),
            (
                "binding_statement",
                Role::BindingStatement,
                &self.binding_statement,
            ),
            (
                "binding_signature",
                Role::BindingSignature,
                &self.binding_signature,
            ),
            ("binding_root", Role::BindingRoot, &self.binding_root),
            ("binding_crl", Role::BindingCrl, &self.binding_crl),
            (
                "consent_statement",
                Role::ConsentStatement,
                &self.consent_statement,
            ),
            (
                "consent_signature",
                Role::ConsentSignature,
                &self.consent_signature,
            ),
        ]
    }
}

/// One holder whose document signature and consent the manifest covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    pub id: String,
    pub account_id: String,
    pub binding_id: String,
    /// Unix seconds, `0..=2^53 - 1`.
    pub binding_registered_at: i64,
    pub entries: SignerEntries,
}

/// The parsed manifest; identifiers are lowercase hyphenated UUIDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub deployment_id: String,
    pub case_id: String,
    pub document_id: String,
    pub document_version: u64,
    pub document_sha256: [u8; 32],
    pub operation_id: String,
    /// Sorted by entry name in byte order when parsed; [`encode`] sorts.
    pub artifacts: Vec<Artifact>,
    pub signers: Vec<Signer>,
}

impl Manifest {
    /// The artifact row of `role` holding `bytes`, for the document named
    /// `document`.
    pub fn artifact(document: &str, role: Role, bytes: &[u8]) -> Artifact {
        Artifact {
            entry: role_entry(role, document),
            role,
            sha256: certificate::sha256(bytes),
            size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        }
    }
}

/// The canonical bytes of `manifest`, with the artifacts sorted by entry
/// name in byte order.
///
/// Fails with the finding [`parse`] would report when the result would not
/// be admitted, so the server never writes a manifest the validator rejects.
pub fn encode(manifest: &Manifest) -> Result<Vec<u8>, Finding> {
    let mut artifacts: Vec<&Artifact> = manifest.artifacts.iter().collect();
    artifacts.sort_by(|left, right| left.entry.as_bytes().cmp(right.entry.as_bytes()));
    let text = |value: &str| Value::String(value.to_owned());
    let member = |key: &str, value: Value| (key.to_owned(), value);
    let artifacts = artifacts
        .into_iter()
        .map(|artifact| {
            Value::Object(vec![
                member("entry", text(&artifact.entry)),
                member("role", text(artifact.role.as_str())),
                member("sha256", text(&hex(&artifact.sha256))),
                member("size", Value::Integer(i128::from(artifact.size))),
            ])
        })
        .collect();
    let signers = manifest
        .signers
        .iter()
        .map(|signer| {
            let entries = signer
                .entries
                .members()
                .iter()
                .map(|(key, _, entry)| member(key, text(entry)))
                .collect();
            Value::Object(vec![
                member("id", text(&signer.id)),
                member("account_id", text(&signer.account_id)),
                member("binding_id", text(&signer.binding_id)),
                member(
                    "binding_registered_at",
                    Value::Integer(i128::from(signer.binding_registered_at)),
                ),
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
        member(
            "profile_version",
            Value::Integer(i128::from(PROFILE_VERSION)),
        ),
        member("purpose", text(PURPOSE)),
        member("deployment_id", text(&manifest.deployment_id)),
        member("case_id", text(&manifest.case_id)),
        member("document_id", text(&manifest.document_id)),
        member(
            "document_version",
            Value::Integer(i128::from(manifest.document_version)),
        ),
        member("document_sha256", text(&hex(&manifest.document_sha256))),
        member("operation_id", text(&manifest.operation_id)),
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
/// the JSON code; any member outside the exact schema fails with
/// `manifest.schema`; artifacts out of byte order or repeated fail with
/// their own codes; more than one signer is not supported.
pub fn parse(bytes: &[u8]) -> Result<Manifest, Finding> {
    let value = json::parse_canonical(
        bytes,
        Limits {
            max_values: MAX_VALUES,
        },
    )
    .map_err(json_finding)?;
    schema::manifest(&value)
}

/// A JSON rejection as a failed finding with its own code.
pub(crate) fn json_finding(error: JsonError) -> Finding {
    Finding::failed(error.code(), format!("at byte {}", error.offset))
}
