//! Handler for the `package` command group.
//!
//! The export handler wires the SHA-256 hasher and the stored ZIP
//! writer into the evidence export use case: every artifact is read,
//! the instruction file is generated with the local openssl version,
//! and the archive is written to the requested path.

use std::fs;

use anyhow::Context;
use application::evidence::{EvidenceRequest, ExportEvidencePackage};
use infrastructure::{openssl_version, RingSha256Hasher, StoredZipWriter};

use crate::cli::{PackageAction, PackageExportArgs};

/// Dispatches a `package` subcommand to its handler.
pub fn run(action: PackageAction, json: bool) -> anyhow::Result<()> {
    match action {
        PackageAction::Export(args) => export(&args, json),
    }
}

/// Builds the evidence package and writes it to `--out`.
///
/// Human-readable output is the document digest and the package path.
/// With `json` set, the output is one object carrying the digest, the
/// package path, and the name of the instruction file inside it.
fn export(args: &PackageExportArgs, json: bool) -> anyhow::Result<()> {
    let document = fs::read(&args.file)
        .with_context(|| format!("cannot read document {}", args.file.display()))?;
    let signature = fs::read(&args.sig)
        .with_context(|| format!("cannot read signature {}", args.sig.display()))?;
    let token = fs::read(&args.tsr)
        .with_context(|| format!("cannot read timestamp token {}", args.tsr.display()))?;
    let certificate = fs::read(&args.cert)
        .with_context(|| format!("cannot read certificate {}", args.cert.display()))?;
    let issuer = fs::read(&args.ca)
        .with_context(|| format!("cannot read issuer certificate {}", args.ca.display()))?;
    let crl = fs::read(&args.crl)
        .with_context(|| format!("cannot read revocation list {}", args.crl.display()))?;
    let tsa_chain = args
        .tsa_chain
        .as_ref()
        .map(|path| fs::read(path).with_context(|| format!("cannot read {}", path.display())))
        .transpose()?;

    let document_name = args
        .file
        .file_name()
        .and_then(|name| name.to_str())
        .context("the document path must end in a utf-8 file name")?;
    let version = openssl_version().context("cannot determine the local openssl version")?;

    let use_case = ExportEvidencePackage::new(RingSha256Hasher::new(), StoredZipWriter::new());
    let package = use_case
        .execute(&EvidenceRequest {
            document_name,
            document: &document,
            signature: &signature,
            timestamp_token: &token,
            signer_certificate_pem: &certificate,
            issuer_certificate_pem: &issuer,
            crl_pem: &crl,
            tsa_chain_pem: tsa_chain.as_deref(),
            openssl_version: &version,
        })
        .context("the evidence package could not be built")?;

    fs::write(&args.out, &package.archive)
        .with_context(|| format!("cannot write {}", args.out.display()))?;

    if json {
        let body = serde_json::json!({
            "digest": package.document_digest_hex,
            "package": args.out.display().to_string(),
            "instructions": application::evidence::INSTRUCTIONS_ENTRY_NAME,
        });
        println!("{body}");
    } else {
        println!("digest:  {}", package.document_digest_hex);
        println!("package: {}", args.out.display());
        println!(
            "instructions inside the package: {}",
            application::evidence::INSTRUCTIONS_ENTRY_NAME
        );
    }
    Ok(())
}
