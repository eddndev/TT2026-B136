//! Handler for the `verify` command.
//!
//! Wires the SHA-256 hasher, the RSA signature verifier, the X.509
//! chain validator, and the RFC 3161 token verifier into the integral
//! verification use case, then renders the four-component report. The
//! process exits nonzero when the verdict is not valid, so scripts can
//! branch on the outcome directly.

use std::fs::{self, File};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use application::verification::{
    ComponentReport, TimestampEvidence, Verdict, VerifyDocument, VerifyDocumentRequest,
};
use domain::crypto::Signature;
use infrastructure::{Rfc3161Verifier, RingSha256Hasher, RsaPkcs1Verifier, X509ChainValidator};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::VerifyArgs;

/// Verifies a document and prints the component report.
///
/// Human-readable output is one line per component plus the verdict.
/// With `json` set, the output is one object carrying the digest, the
/// four components, and the verdict.
pub fn run(args: &VerifyArgs, json: bool) -> anyhow::Result<()> {
    let signature = Signature::from_bytes(
        fs::read(&args.sig).with_context(|| format!("cannot read {}", args.sig.display()))?,
    )
    .with_context(|| format!("{} does not hold a signature", args.sig.display()))?;
    let certificate = fs::read(&args.cert)
        .with_context(|| format!("cannot read certificate {}", args.cert.display()))?;
    let issuer = fs::read(&args.ca)
        .with_context(|| format!("cannot read issuer certificate {}", args.ca.display()))?;
    let crl = args
        .crl
        .as_ref()
        .map(|path| fs::read(path).with_context(|| format!("cannot read {}", path.display())))
        .transpose()?;
    let token = args
        .tsr
        .as_ref()
        .map(|path| fs::read(path).with_context(|| format!("cannot read {}", path.display())))
        .transpose()?;
    let anchor = args
        .tsa_cert
        .as_ref()
        .map(|path| fs::read(path).with_context(|| format!("cannot read {}", path.display())))
        .transpose()?;

    let evaluation_unix = match &args.at {
        Some(text) => OffsetDateTime::parse(text, &Rfc3339)
            .with_context(|| format!("--at must be an rfc 3339 instant, got {text:?}"))?
            .unix_timestamp(),
        None => now_unix()?,
    };

    let timestamp = token.as_ref().map(|token| TimestampEvidence {
        token,
        // The internal timestamp authority chains to the same root as
        // the signer, so the issuer certificate is the default anchor.
        trust_anchor_pem: anchor.as_deref().unwrap_or(&issuer),
    });
    let request = VerifyDocumentRequest {
        signature: &signature,
        signer_certificate_pem: &certificate,
        issuer_certificate_pem: &issuer,
        crl_pem: crl.as_deref(),
        timestamp,
        evaluation_unix,
    };

    let mut document = File::open(&args.file)
        .with_context(|| format!("cannot open file: {}", args.file.display()))?;
    let use_case = VerifyDocument::new(
        RingSha256Hasher::new(),
        RsaPkcs1Verifier::new(),
        X509ChainValidator::new(),
        Rfc3161Verifier::new(),
    );
    let report = use_case
        .execute(&mut document, &request)
        .context("verification could not be completed")?;

    if json {
        let body = serde_json::json!({
            "document": args.file.display().to_string(),
            "digest": report.document_digest_hex,
            "integrity": component_json(&report.integrity),
            "signature": component_json(&report.signature),
            "certificate": component_json(&report.certificate),
            "timestamp": component_json(&report.timestamp),
            "verdict": report.verdict.to_string(),
        });
        println!("{body}");
    } else {
        println!("document:    {}", args.file.display());
        println!("digest:      {}", report.document_digest_hex);
        print_component("integrity", &report.integrity);
        print_component("signature", &report.signature);
        print_component("certificate", &report.certificate);
        print_component("timestamp", &report.timestamp);
        println!("verdict:     {}", report.verdict);
    }

    if report.verdict != Verdict::Valid {
        anyhow::bail!("the verification verdict is not valid");
    }
    Ok(())
}

fn print_component(name: &str, component: &ComponentReport) {
    // The name column is padded so the status column lines up across
    // the four components ("certificate" is the longest name).
    println!(
        "{name}:{padding} {status}: {detail}",
        padding = " ".repeat(11 - name.len()),
        status = component.status,
        detail = component.detail
    );
}

fn component_json(component: &ComponentReport) -> serde_json::Value {
    serde_json::json!({
        "status": component.status.to_string(),
        "detail": component.detail,
    })
}

fn now_unix() -> anyhow::Result<i64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?;
    Ok(now.as_secs() as i64)
}
