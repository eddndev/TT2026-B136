//! End-to-end tests of the OpenSSL-script-backed authority adapter,
//! driving the real scripts under `pki/` against a temporary CA
//! directory.

use std::path::{Path, PathBuf};

use domain::crypto::certificate::{
    CertificateAuthority, CertificateValidation, CertificateValidator,
};
use domain::DomainError;
use infrastructure::{OpensslCaAdapter, X509ChainValidator};

fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

fn adapter_in(ca_dir: &Path) -> OpensslCaAdapter {
    OpensslCaAdapter::new(scripts_dir(), ca_dir)
}

#[test]
fn init_ca_creates_the_root_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = adapter_in(&dir.path().join("ca"));

    let root = adapter.init_ca().unwrap();
    assert!(root.starts_with(b"-----BEGIN CERTIFICATE-----"));

    let again = adapter.init_ca().unwrap();
    assert_eq!(root, again, "re-running init must not replace the root");
    assert_eq!(adapter.root_certificate().unwrap(), root);
}

#[test]
fn issued_certificates_chain_to_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = adapter_in(&dir.path().join("ca"));
    let root = adapter.init_ca().unwrap();

    let issued = adapter.issue("Ana Prueba").unwrap();
    assert_eq!(issued.serial_hex, "1000", "the serial file starts at 1000");
    assert!(issued.certificate_path.is_file());
    assert!(issued.private_key_path.is_file());

    let validator = X509ChainValidator::new();
    let summary = validator.inspect(&issued.certificate_pem).unwrap();
    assert!(summary.subject.contains("Ana Prueba"));

    // One hour into the certificate's window: the issuing script backdates
    // the certificate's notBefore by five minutes, so an instant that close
    // to it would precede the root's own notBefore and fail validation.
    let outcome = validator
        .validate(
            &issued.certificate_pem,
            &root,
            None,
            summary.not_before_unix + 3_600,
        )
        .unwrap();
    assert_eq!(outcome, CertificateValidation::Valid);
}

#[test]
fn revoke_then_generate_crl_marks_the_certificate_revoked() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = adapter_in(&dir.path().join("ca"));
    let root = adapter.init_ca().unwrap();
    let issued = adapter.issue("Baja Inmediata").unwrap();

    // Serials are normalized, so the lowercase form addresses the same
    // certificate.
    adapter.revoke(&issued.serial_hex.to_lowercase()).unwrap();
    let crl = adapter.generate_crl().unwrap();
    assert!(crl.starts_with(b"-----BEGIN X509 CRL-----"));

    let validator = X509ChainValidator::new();
    let summary = validator.inspect(&issued.certificate_pem).unwrap();
    // One hour into the certificate's window, inside the root's window as
    // well despite the five-minute backdating of the certificate's
    // notBefore, and well within the revocation list's seven-day period.
    let outcome = validator
        .validate(
            &issued.certificate_pem,
            &root,
            Some(&crl),
            summary.not_before_unix + 3_600,
        )
        .unwrap();
    assert_eq!(
        outcome,
        CertificateValidation::Revoked {
            serial_hex: issued.serial_hex,
        }
    );
}

#[test]
fn issuing_with_a_common_name_the_scripts_refuse_surfaces_their_error() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = adapter_in(&dir.path().join("ca"));
    adapter.init_ca().unwrap();

    let err = adapter.issue("nombre/invalido").unwrap_err();
    match err {
        DomainError::CertificateAuthorityFailure(message) => {
            assert!(
                message.contains("common name"),
                "the script's own diagnosis should be preserved, got: {message}"
            );
        }
        other => panic!("expected an authority failure, got {other:?}"),
    }
}

#[test]
fn issuing_before_initialization_fails() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = adapter_in(&dir.path().join("ca"));

    let err = adapter.issue("Sin Autoridad").unwrap_err();
    match err {
        DomainError::CertificateAuthorityFailure(message) => {
            assert!(
                message.contains("CA not initialized"),
                "the script's own diagnosis should be preserved, got: {message}"
            );
        }
        other => panic!("expected an authority failure, got {other:?}"),
    }
}
