#![cfg(not(target_arch = "wasm32"))]

mod support;

use browser_crypto::{check_key, generate, inspect, sign};
use der::{Decode, DecodePem, Encode, EncodePem};
use pkcs5::pbes2::{EncryptionScheme, Kdf, Pbkdf2Prf};
use pkcs8::EncryptedPrivateKeyInfo;
use sha2::{Digest, Sha256};
use support::{fixture, openssl, write_private, PASSWORD};
use x509_cert::{request::CertReq, Certificate};

#[test]
fn generated_encrypted_backup_and_csr_are_interoperable_with_openssl() {
    let material = fixture();
    let (_, encrypted_der) = pkcs8::SecretDocument::from_pem(&material.key).unwrap();
    let encrypted = EncryptedPrivateKeyInfo::from_der(encrypted_der.as_bytes()).unwrap();
    let parameters = encrypted.encryption_algorithm.pbes2().unwrap();
    let Kdf::Pbkdf2(parameters_kdf) = &parameters.kdf else {
        panic!("export must use PBKDF2");
    };
    assert_eq!(parameters_kdf.iteration_count, 600_000);
    assert_eq!(parameters_kdf.prf, Pbkdf2Prf::HmacWithSha256);
    assert_eq!(parameters_kdf.salt.len(), 16);
    assert_eq!(parameters.encryption.key_size(), 32);
    let EncryptionScheme::Aes256Cbc { iv } = parameters.encryption else {
        panic!("export must use AES-256-CBC");
    };
    assert_eq!(iv.len(), 16);
    assert!(material
        .key
        .starts_with("-----BEGIN ENCRYPTED PRIVATE KEY-----"));
    assert!(check_key(&material.key, PASSWORD, &material.csr).unwrap());
    openssl(
        material.directory.path(),
        &["req", "-in", "request.pem", "-verify", "-noout"],
    );
    openssl(
        material.directory.path(),
        &[
            "pkey",
            "-in",
            "key.pem",
            "-passin",
            "file:password",
            "-check",
            "-noout",
        ],
    );
    assert_eq!(inspect(&material.cert).unwrap().rsa_bits, 3072);
}

#[test]
fn signature_covers_exact_bytes_and_openssl_verifies_it() {
    let material = fixture();
    let bytes = b"QADRA\0document-proof\r\nexact UTF-8: \xc3\xb1\n";
    let signature = sign(&material.key, PASSWORD, &material.cert, bytes).unwrap();
    assert_eq!(signature.len(), 384);
    let scratch = tempfile::tempdir().unwrap();
    write_private(scratch.path(), "certificate.der", &material.cert);
    write_private(scratch.path(), "message", bytes);
    write_private(scratch.path(), "signature", &signature);
    openssl(
        scratch.path(),
        &[
            "x509",
            "-inform",
            "DER",
            "-in",
            "certificate.der",
            "-pubkey",
            "-noout",
            "-out",
            "public.pem",
        ],
    );
    openssl(
        scratch.path(),
        &[
            "dgst",
            "-sha256",
            "-verify",
            "public.pem",
            "-signature",
            "signature",
            "message",
        ],
    );
    write_private(scratch.path(), "message", b"changed");
    let result = support::openssl_output(
        scratch.path(),
        &[
            "dgst",
            "-sha256",
            "-verify",
            "public.pem",
            "-signature",
            "signature",
            "message",
        ],
    );
    assert!(!result.status.success(), "changed bytes must not verify");
}

#[test]
fn certificate_inspection_returns_public_exact_fingerprints() {
    let material = fixture();
    let info = inspect(&material.cert).unwrap();
    let parsed = Certificate::from_der(&material.cert).unwrap();
    let spki = parsed
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .unwrap();
    assert_eq!(
        info.certificate_sha256,
        format!("{:x}", Sha256::digest(&material.cert))
    );
    assert_eq!(info.spki_sha256, format!("{:x}", Sha256::digest(spki)));
    assert!(info.subject.contains("Browser holder"));
    assert!(info.subject.contains("Example Office"));
}

#[test]
fn wrong_password_malformed_inputs_and_tampered_csr_are_rejected() {
    let material = fixture();
    assert!(sign(&material.key, "incorrect", &material.cert, b"proof").is_err());
    assert!(check_key(&material.key, "incorrect", &material.csr).is_err());
    assert!(sign("not a key", PASSWORD, &material.cert, b"proof").is_err());
    assert!(inspect(b"not a certificate").is_err());
    let request = CertReq::from_pem(&material.csr).unwrap();
    let mut encoded = request.to_der().unwrap();
    *encoded.last_mut().unwrap() ^= 1;
    let altered = CertReq::from_der(&encoded)
        .unwrap()
        .to_pem(der::pem::LineEnding::LF)
        .unwrap();
    assert!(check_key(&material.key, PASSWORD, &altered).is_err());
}

#[test]
fn existing_openssl_pbkdf2_2048_iteration_keys_remain_usable() {
    let material = fixture();
    let scratch = tempfile::tempdir().unwrap();
    write_private(scratch.path(), "input.pem", material.key.as_bytes());
    write_private(scratch.path(), "password", PASSWORD.as_bytes());
    write_private(scratch.path(), "output-password", PASSWORD.as_bytes());
    openssl(
        scratch.path(),
        &[
            "pkcs8",
            "-topk8",
            "-in",
            "input.pem",
            "-passin",
            "file:password",
            "-passout",
            "file:output-password",
            "-v2",
            "aes-256-cbc",
            "-v2prf",
            "hmacWithSHA256",
            "-iter",
            "2048",
            "-out",
            "legacy.pem",
        ],
    );
    let legacy = std::fs::read_to_string(scratch.path().join("legacy.pem")).unwrap();
    assert!(check_key(&legacy, PASSWORD, &material.csr).unwrap());
    assert_eq!(
        sign(&legacy, PASSWORD, &material.cert, b"legacy proof")
            .unwrap()
            .len(),
        384
    );
}

#[test]
fn rejected_parameters_do_not_start_key_generation_or_decryption() {
    assert!(generate("", "Holder", "Office", "MX").is_err());
    assert!(generate("short", "Holder", "Office", "MX").is_err());
    assert!(generate(&"a".repeat(1025), "Holder", "Office", "MX").is_err());
    assert!(generate(PASSWORD, "", "Office", "MX").is_err());
    assert!(generate(PASSWORD, "Holder", "", "MX").is_err());
    assert!(generate(PASSWORD, "Holder", "Office", "MEX").is_err());
    assert!(sign(&"x".repeat(65537), PASSWORD, &[], &[]).is_err());
    assert!(inspect(&vec![0; 16385]).is_err());
    assert!(check_key("", PASSWORD, &"x".repeat(16385)).is_err());
}

#[test]
fn a_valid_certificate_or_csr_for_another_key_is_rejected() {
    let material = fixture();
    let scratch = tempfile::tempdir().unwrap();
    write_private(scratch.path(), "password", PASSWORD.as_bytes());
    write_private(scratch.path(), "other-key.pem", b"");
    openssl(
        scratch.path(),
        &[
            "req",
            "-new",
            "-x509",
            "-newkey",
            "rsa:3072",
            "-sha256",
            "-days",
            "1",
            "-passout",
            "file:password",
            "-keyout",
            "other-key.pem",
            "-subj",
            "/CN=Other holder",
            "-outform",
            "DER",
            "-out",
            "other-cert.der",
        ],
    );
    openssl(
        scratch.path(),
        &[
            "req",
            "-new",
            "-sha256",
            "-key",
            "other-key.pem",
            "-passin",
            "file:password",
            "-subj",
            "/CN=Other holder",
            "-out",
            "other-request.pem",
        ],
    );
    let certificate = std::fs::read(scratch.path().join("other-cert.der")).unwrap();
    let request = std::fs::read_to_string(scratch.path().join("other-request.pem")).unwrap();
    assert_eq!(
        sign(&material.key, PASSWORD, &certificate, b"proof")
            .unwrap_err()
            .0,
        "key_mismatch"
    );
    assert_eq!(
        check_key(&material.key, PASSWORD, &request).unwrap_err().0,
        "key_mismatch"
    );
}

#[test]
fn oversized_kdf_cost_is_rejected_before_password_derivation() {
    let material = fixture();
    let (_, encoded) = pkcs8::SecretDocument::from_pem(&material.key).unwrap();
    let mut encrypted = EncryptedPrivateKeyInfo::from_der(encoded.as_bytes()).unwrap();
    let pkcs5::EncryptionScheme::Pbes2(parameters) = &mut encrypted.encryption_algorithm else {
        panic!("synthetic fixture must use PBES2");
    };
    let Kdf::Pbkdf2(kdf) = &mut parameters.kdf else {
        panic!("synthetic fixture must use PBKDF2");
    };
    kdf.iteration_count = 2_000_001;
    let altered = pkcs8::SecretDocument::try_from(encrypted.to_der().unwrap())
        .unwrap()
        .to_pem("ENCRYPTED PRIVATE KEY", der::pem::LineEnding::LF)
        .unwrap();
    assert_eq!(
        check_key(&altered, PASSWORD, &material.csr).unwrap_err().0,
        "unsupported_key_encryption"
    );
}
