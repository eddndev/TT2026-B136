//! Handler for the `sign` command.
//!
//! Wires the SHA-256 hasher and the RSA signing adapters into the signing
//! use cases: the document is signed with the private key, the signature is
//! cross-checked against the certificate, and only then is the raw
//! signature written to `<file>.sig`. The cross-check catches a key that
//! does not belong to the presented certificate before anything is written.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use anyhow::Context;
use application::signing::{SignDocument, VerifySignature};
use domain::crypto::SignatureVerification;
use infrastructure::{certificate_subject, RingSha256Hasher, RsaPkcs1Signer, RsaPkcs1Verifier};
use zeroize::Zeroizing;

use crate::cli::SignArgs;

/// Signs a file and writes the raw signature next to it as `<file>.sig`.
///
/// Human-readable output is the digest hex and the signature path. With
/// `json` set, the output is one object carrying the digest, the signature
/// path, and the certificate subject.
pub fn run(args: &SignArgs, json: bool) -> anyhow::Result<()> {
    let key_pem = Zeroizing::new(
        fs::read(&args.key)
            .with_context(|| format!("cannot read private key {}", args.key.display()))?,
    );
    let cert_pem = fs::read(&args.cert)
        .with_context(|| format!("cannot read certificate {}", args.cert.display()))?;

    let signer = RsaPkcs1Signer::new(key_pem)
        .with_context(|| format!("cannot load private key {}", args.key.display()))?;
    let mut input = File::open(&args.file)
        .with_context(|| format!("cannot open file: {}", args.file.display()))?;
    let signed = SignDocument::new(RingSha256Hasher::new(), signer)
        .execute(&mut input)
        .context("signing failed")?;

    // Re-read the document and verify the fresh signature against the
    // certificate, so a key that does not match the certificate fails here
    // instead of producing a signature nobody can verify.
    let mut reread = File::open(&args.file)
        .with_context(|| format!("cannot reopen file: {}", args.file.display()))?;
    let outcome = VerifySignature::new(RingSha256Hasher::new(), RsaPkcs1Verifier::new())
        .execute(&mut reread, &signed.signature, &cert_pem)
        .context("verification against the certificate failed")?;
    if let SignatureVerification::Invalid(cause) = outcome {
        anyhow::bail!(
            "the private key {} does not match the certificate {}: {cause}",
            args.key.display(),
            args.cert.display()
        );
    }

    let sig_path = path_with_sig_suffix(&args.file);
    fs::write(&sig_path, signed.signature.as_bytes())
        .with_context(|| format!("cannot write {}", sig_path.display()))?;

    if json {
        let body = serde_json::json!({
            "digest": signed.digest.to_hex(),
            "signature_path": sig_path.display().to_string(),
            "certificate_subject": certificate_subject(&cert_pem)
                .with_context(|| format!("cannot read certificate {}", args.cert.display()))?,
        });
        println!("{body}");
    } else {
        println!("digest: {}", signed.digest.to_hex());
        println!("signature: {}", sig_path.display());
    }
    Ok(())
}

/// Appends `.sig` to the full file name, keeping the original extension.
fn path_with_sig_suffix(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".sig");
    PathBuf::from(name)
}
