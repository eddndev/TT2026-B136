//! Assemble the existing document cryptography and its local evidence material.

use super::inputs::read;
use anyhow::Context;
use application::documents::{DocumentProcessor, DocumentProcessorPorts, EvidenceMaterial};
use infrastructure::{
    openssl_version, EnvelopeKeyManager, LocalOpensslTsa, Rfc3161Verifier, RingAesGcmCipher,
    RingSha256Hasher, RsaPkcs1Signer, RsaPkcs1Verifier, StoredZipWriter, X509ChainValidator,
};
use zeroize::Zeroizing;

pub(super) fn open(
    args: &crate::serve_args::ServeArgs,
    kek: Zeroizing<Vec<u8>>,
) -> anyhow::Result<DocumentProcessor> {
    let signer_certificate = read(&args.signer_cert, "signer certificate")?;
    let signer_key = Zeroizing::new(read(&args.signer_key, "signer private key")?);
    let issuer_certificate = read(&args.ca_cert, "issuer certificate")?;
    let crl = read(&args.crl, "certificate revocation list")?;
    let tsa_chain = read(
        &args.tsa_dir.join("tsa-chain.pem"),
        "timestamp authority chain",
    )?;
    let signer = RsaPkcs1Signer::new(signer_key).context("cannot load signer private key")?;
    let ports = DocumentProcessorPorts {
        hasher: Box::new(RingSha256Hasher::new()),
        cipher: Box::new(RingAesGcmCipher::new()),
        keys: Box::new(EnvelopeKeyManager::new()),
        signer: Box::new(signer),
        timestamp_service: Box::new(LocalOpensslTsa::new(&args.tsa_config, &args.tsa_dir)),
        signature_verifier: Box::new(RsaPkcs1Verifier::new()),
        certificate_validator: Box::new(X509ChainValidator::new()),
        timestamp_verifier: Box::new(Rfc3161Verifier::new()),
        archiver: Box::new(StoredZipWriter::new()),
    };
    let material = EvidenceMaterial {
        signer_certificate_pem: signer_certificate,
        issuer_certificate_pem: issuer_certificate,
        crl_pem: crl,
        tsa_chain_pem: Some(tsa_chain),
        openssl_version: openssl_version().context("cannot inspect openssl version")?,
    };
    DocumentProcessor::new(ports, material, kek).context("cannot initialize document cryptography")
}
