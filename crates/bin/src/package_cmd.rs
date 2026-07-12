//! Handler for the `package` command group.
//!
//! The export handler wires the SHA-256 hasher and the stored ZIP
//! writer into the evidence export use case: every artifact is read,
//! verified against the others, the instruction file is generated with
//! the local openssl version, and the archive is written to the
//! requested path. Artifacts that do not verify against each other are
//! refused, so an exported package can never fail the checks its own
//! instructions document.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use application::evidence::{EvidenceRequest, ExportEvidencePackage};
use application::verification::{
    ComponentReport, ComponentStatus, TimestampEvidence, Verdict, VerifyDocument,
    VerifyDocumentRequest,
};
use domain::crypto::Signature;
use infrastructure::{
    openssl_version, Rfc3161Verifier, RingSha256Hasher, RsaPkcs1Verifier, StoredZipWriter,
    X509ChainValidator,
};

use crate::cli::{PackageAction, PackageExportArgs};

/// Dispatches a `package` subcommand to its handler.
pub fn run(action: PackageAction, json: bool) -> anyhow::Result<()> {
    match action {
        PackageAction::Export(args) => export(&args, json),
    }
}

/// Builds the evidence package and writes it to `--out`.
///
/// The artifacts are verified against each other first; when the
/// verdict is not valid the export is refused, no archive is written,
/// and the error names every failing component.
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

    verify_artifacts(
        &document,
        &signature,
        &certificate,
        &issuer,
        &crl,
        &token,
        tsa_chain.as_deref(),
    )?;

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

/// Runs the same verification the `verify` command performs over the
/// artifacts about to be bundled and rejects any set whose verdict is
/// not valid, naming every failing component.
///
/// Without this gate the tool could produce a package that fails the
/// very openssl checks its generated instructions document, for
/// example when the revocation list comes from a different authority
/// than the issuer certificate.
fn verify_artifacts(
    document: &[u8],
    signature_bytes: &[u8],
    signer_certificate_pem: &[u8],
    issuer_certificate_pem: &[u8],
    crl_pem: &[u8],
    token: &[u8],
    tsa_chain_pem: Option<&[u8]>,
) -> anyhow::Result<()> {
    let signature = Signature::from_bytes(signature_bytes.to_vec())
        .context("the supplied signature file does not hold a signature")?;
    let request = VerifyDocumentRequest {
        signature: &signature,
        signer_certificate_pem,
        issuer_certificate_pem,
        crl_pem: Some(crl_pem),
        timestamp: Some(TimestampEvidence {
            token,
            // The instructions anchor the token check on the bundled
            // TSA chain when one travels in the package and on the
            // issuer root otherwise; the pre-export check must judge
            // the token against the same anchor.
            trust_anchor_pem: tsa_chain_pem.unwrap_or(issuer_certificate_pem),
        }),
        evaluation_unix: now_unix()?,
    };

    let use_case = VerifyDocument::new(
        RingSha256Hasher::new(),
        RsaPkcs1Verifier::new(),
        X509ChainValidator::new(),
        Rfc3161Verifier::new(),
    );
    let report = use_case
        .execute(&mut &document[..], &request)
        .context("the artifacts could not be verified before export")?;
    if report.verdict != Verdict::Valid {
        let components = [
            ("integrity", &report.integrity),
            ("signature", &report.signature),
            ("certificate", &report.certificate),
            ("timestamp", &report.timestamp),
        ];
        let failures: Vec<String> = components
            .iter()
            .filter(|(_, component)| component.status == ComponentStatus::Failed)
            .map(|(name, component): &(&str, &ComponentReport)| {
                format!("{name} ({})", component.detail)
            })
            .collect();
        anyhow::bail!(
            "refusing to export: the package would fail its own verification \
             instructions; failing component(s): {}",
            failures.join("; ")
        );
    }
    Ok(())
}

fn now_unix() -> anyhow::Result<i64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?;
    Ok(now.as_secs() as i64)
}
