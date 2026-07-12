//! Use case: export a self-contained evidence package.
//!
//! The package is one archive holding everything a third party needs to
//! verify a signed and timestamped document with nothing but the
//! openssl command line tool: the document, its detached signature, its
//! timestamp token, the signer and issuer certificates, the revocation
//! list, optionally the timestamp authority's certificate chain, and a
//! generated INSTRUCCIONES.md walking through the verification
//! commands. The archive container is chosen by the adapter behind the
//! archive port; see docs/adr/0006-evidence-package-format.md.

use domain::crypto::archive::{ArchiveEntry, ArchiveWriter};
use domain::crypto::DocumentHasher;

use crate::error::ApplicationError;

/// Verification instructions template. The Spanish prose lives in a
/// versioned Markdown file; placeholders are filled at export time.
const INSTRUCTIONS_TEMPLATE: &str = include_str!("evidence/INSTRUCCIONES.template.md");

/// Entry name of the generated instruction file.
pub const INSTRUCTIONS_ENTRY_NAME: &str = "INSTRUCCIONES.md";

/// Entry name of the signer certificate.
pub const SIGNER_CERTIFICATE_ENTRY_NAME: &str = "certificado.pem";

/// Entry name of the issuing authority certificate.
pub const ISSUER_CERTIFICATE_ENTRY_NAME: &str = "ca.pem";

/// Entry name of the certificate revocation list.
pub const CRL_ENTRY_NAME: &str = "crl.pem";

/// Entry name of the timestamp authority chain, when available.
pub const TSA_CHAIN_ENTRY_NAME: &str = "tsa-chain.pem";

/// Everything one export run bundles. The document's base name defines
/// the names of the document, signature, and token entries; the
/// remaining entries carry fixed names that the instruction template
/// references verbatim.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceRequest<'a> {
    /// Base name the document keeps inside the archive.
    pub document_name: &'a str,
    /// The document bytes.
    pub document: &'a [u8],
    /// Detached signature over the document digest.
    pub signature: &'a [u8],
    /// RFC 3161 timestamp token over the document digest.
    pub timestamp_token: &'a [u8],
    /// PEM certificate of the signer.
    pub signer_certificate_pem: &'a [u8],
    /// PEM certificate of the issuing authority.
    pub issuer_certificate_pem: &'a [u8],
    /// PEM revocation list current at export time.
    pub crl_pem: &'a [u8],
    /// PEM chain of the timestamp authority, when available.
    pub tsa_chain_pem: Option<&'a [u8]>,
    /// Version line of the local openssl tool, quoted in the
    /// instructions so a reader knows what the commands were written
    /// against.
    pub openssl_version: &'a str,
}

/// A built evidence package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidencePackage {
    /// The archive bytes, ready to be written to disk.
    pub archive: Vec<u8>,
    /// Lower-case hex of the document's SHA-256 digest, as recorded in
    /// the instructions.
    pub document_digest_hex: String,
    /// The generated instruction text, as placed inside the archive.
    pub instructions: String,
}

/// Builds evidence packages over the hashing and archive ports.
pub struct ExportEvidencePackage<H, A> {
    hasher: H,
    archiver: A,
}

impl<H: DocumentHasher, A: ArchiveWriter> ExportEvidencePackage<H, A> {
    /// Builds the use case over a hashing port and an archive port.
    pub fn new(hasher: H, archiver: A) -> Self {
        Self { hasher, archiver }
    }

    /// Digests the document, fills the instruction template, and
    /// archives every artifact.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::Domain`] when the document name is
    /// not usable as an archive entry name or the archive cannot be
    /// written.
    pub fn execute(
        &self,
        request: &EvidenceRequest<'_>,
    ) -> Result<EvidencePackage, ApplicationError> {
        let digest_hex = self.hasher.hash_bytes(request.document).to_hex();
        let signature_name = format!("{}.sig", request.document_name);
        let token_name = format!("{}.tsr", request.document_name);

        let instructions = INSTRUCTIONS_TEMPLATE
            .replace("{{DOCUMENTO}}", request.document_name)
            .replace("{{FIRMA}}", &signature_name)
            .replace("{{SELLO}}", &token_name)
            .replace("{{DIGEST_SHA256}}", &digest_hex)
            .replace("{{OPENSSL_VERSION}}", request.openssl_version);

        let mut entries = vec![
            ArchiveEntry::new(request.document_name, request.document.to_vec())?,
            ArchiveEntry::new(signature_name, request.signature.to_vec())?,
            ArchiveEntry::new(token_name, request.timestamp_token.to_vec())?,
            ArchiveEntry::new(
                SIGNER_CERTIFICATE_ENTRY_NAME,
                request.signer_certificate_pem.to_vec(),
            )?,
            ArchiveEntry::new(
                ISSUER_CERTIFICATE_ENTRY_NAME,
                request.issuer_certificate_pem.to_vec(),
            )?,
            ArchiveEntry::new(CRL_ENTRY_NAME, request.crl_pem.to_vec())?,
        ];
        if let Some(chain) = request.tsa_chain_pem {
            entries.push(ArchiveEntry::new(TSA_CHAIN_ENTRY_NAME, chain.to_vec())?);
        }
        entries.push(ArchiveEntry::new(
            INSTRUCTIONS_ENTRY_NAME,
            instructions.clone().into_bytes(),
        )?);

        let archive = self.archiver.write_archive(&entries)?;
        Ok(EvidencePackage {
            archive,
            document_digest_hex: digest_hex,
            instructions,
        })
    }
}
