//! Signing tests against RSA keys generated with the openssl command line,
//! the same tooling the pki/ scripts use to issue signer credentials.

use std::path::{Path, PathBuf};
use std::process::Command;

use domain::crypto::{
    DocumentHasher, Sha256Digest, Signature, SignatureRejection, SignatureVerification,
};
use infrastructure::{RingSha256Hasher, RsaPkcs1Signer, RsaPkcs1Verifier};
use zeroize::Zeroizing;

/// Runs openssl with `args`, failing the test on a non-zero exit.
fn openssl(args: &[&str], when: &str) {
    let output = Command::new("openssl")
        .args(args)
        .output()
        .expect("openssl must be runnable");
    assert!(
        output.status.success(),
        "{when} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Generates an RSA private key of `bits` as a PKCS#8 PEM file.
fn generate_key(dir: &Path, name: &str, bits: u32) -> PathBuf {
    let path = dir.join(name);
    let bits_opt = format!("rsa_keygen_bits:{bits}");
    openssl(
        &[
            "genpkey",
            "-quiet",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            &bits_opt,
            "-out",
            path.to_str().unwrap(),
        ],
        "key generation",
    );
    path
}

/// Extracts the SubjectPublicKeyInfo public key PEM from a private key.
fn extract_public_key(dir: &Path, key: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    openssl(
        &[
            "pkey",
            "-in",
            key.to_str().unwrap(),
            "-pubout",
            "-out",
            path.to_str().unwrap(),
        ],
        "public key extraction",
    );
    path
}

/// Issues a self-signed certificate for `key` with the given common name.
fn self_signed_cert(dir: &Path, key: &Path, cn: &str, name: &str) -> PathBuf {
    let path = dir.join(name);
    let subject = format!("/CN={cn}");
    openssl(
        &[
            "req",
            "-x509",
            "-key",
            key.to_str().unwrap(),
            "-subj",
            &subject,
            "-days",
            "1",
            "-sha256",
            "-out",
            path.to_str().unwrap(),
        ],
        "certificate issuance",
    );
    path
}

fn load_signer(key: &Path) -> Result<RsaPkcs1Signer, infrastructure::CryptoError> {
    let pem = Zeroizing::new(std::fs::read(key).expect("key file is readable"));
    RsaPkcs1Signer::new(pem)
}

fn digest_of(data: &[u8]) -> Sha256Digest {
    RingSha256Hasher::new().hash_bytes(data)
}

fn sample_document() -> Vec<u8> {
    (0..4096u32).map(|i| (i % 253) as u8).collect()
}

#[test]
fn a_2048_bit_key_is_rejected_by_the_signer_constructor() {
    use domain::crypto::DocumentSigner;

    let dir = tempfile::tempdir().unwrap();
    let key = generate_key(dir.path(), "small.key.pem", 2048);
    let err = load_signer(&key).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("3072") && message.contains("2048"),
        "the error must name both the required and the actual size: {message}"
    );

    // A conforming key is accepted and signs.
    let key = generate_key(dir.path(), "ok.key.pem", 3072);
    let signer = load_signer(&key).unwrap();
    let signature = signer.sign(&digest_of(b"accepted")).unwrap();
    assert_eq!(signature.as_bytes().len(), 3072 / 8);
}

#[test]
fn pkcs1_and_pkcs8_encodings_of_one_key_sign_identically() {
    use domain::crypto::DocumentSigner;

    let dir = tempfile::tempdir().unwrap();
    let pkcs8 = generate_key(dir.path(), "signer.key.pem", 3072);
    let pkcs1 = dir.path().join("signer.pkcs1.pem");
    openssl(
        &[
            "rsa",
            "-in",
            pkcs8.to_str().unwrap(),
            "-traditional",
            "-out",
            pkcs1.to_str().unwrap(),
        ],
        "pkcs#1 re-encoding",
    );
    let header = std::fs::read_to_string(&pkcs1).unwrap();
    assert!(
        header.contains("-----BEGIN RSA PRIVATE KEY-----"),
        "openssl -traditional must emit a pkcs#1 pem"
    );

    // PKCS#1 v1.5 signing is deterministic, so both encodings of the same
    // key must produce byte-identical signatures.
    let digest = digest_of(&sample_document());
    let from_pkcs8 = load_signer(&pkcs8).unwrap().sign(&digest).unwrap();
    let from_pkcs1 = load_signer(&pkcs1).unwrap().sign(&digest).unwrap();
    assert_eq!(from_pkcs8.as_bytes(), from_pkcs1.as_bytes());
}

#[test]
fn signer_output_verifies_with_the_openssl_command_line() {
    use domain::crypto::DocumentSigner;

    let dir = tempfile::tempdir().unwrap();
    let key = generate_key(dir.path(), "signer.key.pem", 3072);
    let public_key = extract_public_key(dir.path(), &key, "signer.pub.pem");

    let document = sample_document();
    let doc_path = dir.path().join("document.bin");
    std::fs::write(&doc_path, &document).unwrap();

    let signature = load_signer(&key)
        .unwrap()
        .sign(&digest_of(&document))
        .unwrap();
    let sig_path = dir.path().join("document.sig");
    std::fs::write(&sig_path, signature.as_bytes()).unwrap();

    let output = Command::new("openssl")
        .args([
            "dgst",
            "-sha256",
            "-verify",
            public_key.to_str().unwrap(),
            "-signature",
            sig_path.to_str().unwrap(),
            doc_path.to_str().unwrap(),
        ])
        .output()
        .expect("openssl must be runnable");
    assert!(
        output.status.success(),
        "openssl rejected the signature: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Verified OK"));
}

/// One key, its certificate, its bare public key, a signed document, and
/// the signature over that document's digest: the fixture every
/// verification test starts from.
struct SignedFixture {
    _dir: tempfile::TempDir,
    dir_path: PathBuf,
    key: PathBuf,
    cert_pem: Vec<u8>,
    public_key_pem: Vec<u8>,
    document: Vec<u8>,
    signature: Signature,
}

fn signed_fixture() -> SignedFixture {
    use domain::crypto::DocumentSigner;

    let dir = tempfile::tempdir().unwrap();
    let key = generate_key(dir.path(), "signer.key.pem", 3072);
    let cert = self_signed_cert(dir.path(), &key, "Test Signer", "signer.crt.pem");
    let public_key = extract_public_key(dir.path(), &key, "signer.pub.pem");
    let document = sample_document();
    let signature = load_signer(&key)
        .unwrap()
        .sign(&digest_of(&document))
        .unwrap();
    SignedFixture {
        dir_path: dir.path().to_path_buf(),
        _dir: dir,
        key,
        cert_pem: std::fs::read(&cert).unwrap(),
        public_key_pem: std::fs::read(&public_key).unwrap(),
        document,
        signature,
    }
}

fn verify(
    digest: &Sha256Digest,
    signature: &Signature,
    key_material: &[u8],
) -> SignatureVerification {
    use domain::crypto::SignatureVerifier;
    RsaPkcs1Verifier::new()
        .verify(digest, signature, key_material)
        .unwrap()
}

#[test]
fn sign_then_verify_is_valid_with_certificate_and_with_public_key() {
    let fixture = signed_fixture();
    let digest = digest_of(&fixture.document);
    assert_eq!(
        verify(&digest, &fixture.signature, &fixture.cert_pem),
        SignatureVerification::Valid
    );
    assert_eq!(
        verify(&digest, &fixture.signature, &fixture.public_key_pem),
        SignatureVerification::Valid
    );
}

#[test]
fn a_changed_document_byte_is_rejected_as_a_mismatch() {
    let fixture = signed_fixture();
    let mut altered = fixture.document.clone();
    altered[1234] ^= 0x01;
    assert_eq!(
        verify(&digest_of(&altered), &fixture.signature, &fixture.cert_pem),
        SignatureVerification::Invalid(SignatureRejection::MismatchedDocumentOrKey)
    );
}

#[test]
fn a_changed_signature_byte_is_rejected_as_a_mismatch() {
    let fixture = signed_fixture();
    let mut altered = fixture.signature.as_bytes().to_vec();
    altered[100] ^= 0x01;
    let altered = Signature::from_bytes(altered).unwrap();
    assert_eq!(
        verify(&digest_of(&fixture.document), &altered, &fixture.cert_pem),
        SignatureVerification::Invalid(SignatureRejection::MismatchedDocumentOrKey)
    );
}

#[test]
fn a_different_certificate_is_rejected_as_a_mismatch() {
    let fixture = signed_fixture();
    let other_key = generate_key(&fixture.dir_path, "other.key.pem", 3072);
    let other_cert = self_signed_cert(
        &fixture.dir_path,
        &other_key,
        "Other Signer",
        "other.crt.pem",
    );
    let other_cert_pem = std::fs::read(&other_cert).unwrap();
    assert_eq!(
        verify(
            &digest_of(&fixture.document),
            &fixture.signature,
            &other_cert_pem
        ),
        SignatureVerification::Invalid(SignatureRejection::MismatchedDocumentOrKey)
    );
}

#[test]
fn an_openssl_signature_verifies_as_valid() {
    let fixture = signed_fixture();
    let doc_path = fixture.dir_path.join("document.bin");
    std::fs::write(&doc_path, &fixture.document).unwrap();
    let sig_path = fixture.dir_path.join("document.openssl.sig");
    openssl(
        &[
            "dgst",
            "-sha256",
            "-sign",
            fixture.key.to_str().unwrap(),
            "-out",
            sig_path.to_str().unwrap(),
            doc_path.to_str().unwrap(),
        ],
        "openssl signing",
    );
    let signature = Signature::from_bytes(std::fs::read(&sig_path).unwrap()).unwrap();
    assert_eq!(
        verify(&digest_of(&fixture.document), &signature, &fixture.cert_pem),
        SignatureVerification::Valid
    );
}

#[test]
fn a_signature_of_the_wrong_length_is_malformed() {
    let fixture = signed_fixture();
    let stub = Signature::from_bytes(vec![1u8; 10]).unwrap();
    let outcome = verify(&digest_of(&fixture.document), &stub, &fixture.cert_pem);
    match outcome {
        SignatureVerification::Invalid(SignatureRejection::MalformedSignature(message)) => {
            assert!(
                message.contains("384") && message.contains("10"),
                "the cause must name the expected and actual lengths: {message}"
            );
        }
        other => panic!("expected a malformed-signature cause, got: {other:?}"),
    }
}

#[test]
fn certificate_subject_reports_the_common_name() {
    let fixture = signed_fixture();
    let subject = infrastructure::signing::certificate_subject(&fixture.cert_pem).unwrap();
    assert!(
        subject.contains("Test Signer"),
        "subject must carry the common name: {subject}"
    );
}

#[test]
fn the_signer_debug_form_withholds_the_private_key() {
    let dir = tempfile::tempdir().unwrap();
    let key = generate_key(dir.path(), "signer.key.pem", 3072);
    let signer = load_signer(&key).unwrap();

    let shown = format!("{signer:?}");
    assert!(
        shown.contains("private key pem withheld"),
        "the debug form must state the key is withheld: {shown}"
    );
    // No substantial line of the actual PEM may appear in the debug output.
    let pem = std::fs::read_to_string(&key).unwrap();
    for line in pem.lines().filter(|line| line.len() > 20) {
        assert!(
            !shown.contains(line),
            "the debug form must not leak key material: {shown}"
        );
    }
}
