#[path = "declaration_cases/certificates.rs"]
mod certificate_cases;
mod declaration_fixture;
#[path = "declaration_cases/revocation.rs"]
mod revocation_cases;

use domain::crypto::{
    CredentialFailure, DocumentHasher, InternalDeclarationVerifier, Sha256Digest, Signature,
};
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::RingSha256Hasher;

use declaration_fixture::fixture;

#[test]
fn openssl_signature_is_admitted_with_captured_public_materials() {
    let fx = fixture();
    let verifier = InternalRsaDeclarationVerifier::new();
    let digest = RingSha256Hasher.hash_bytes(&fx.statement);
    let result = verifier
        .verify(&digest, &fx.leaf, &fx.signature, &fx.root, &fx.crl, fx.at)
        .unwrap();
    assert_eq!(result.statement_digest, digest);
    assert_eq!(result.signature, fx.signature);
    assert_eq!(result.checked_at, fx.at);
    assert!(result.valid_from <= fx.at && result.valid_until >= fx.at);
    assert_eq!(result.certificate.der, fx.leaf_der);
    assert_eq!(
        result.certificate.fingerprint,
        RingSha256Hasher.hash_bytes(&fx.leaf_der)
    );
    assert_eq!(
        result.trust.root_fingerprint,
        RingSha256Hasher.hash_bytes(&result.trust.root_der)
    );
    assert_eq!(
        result.trust.crl_digest,
        RingSha256Hasher.hash_bytes(&result.trust.crl_der)
    );
    assert!(!format!("{result:?}").contains("Synthetic Declarant"));
}

#[test]
fn changing_the_signed_digest_or_signature_is_rejected() {
    let fx = fixture();
    let verifier = InternalRsaDeclarationVerifier::new();
    assert_eq!(
        verifier
            .verify(
                &Sha256Digest::from_array([9; 32]),
                &fx.leaf,
                &fx.signature,
                &fx.root,
                &fx.crl,
                fx.at
            )
            .unwrap_err(),
        CredentialFailure::InvalidSignature
    );
    let mut bytes = fx.signature.as_bytes().to_vec();
    bytes[0] ^= 1;
    let invalid = Signature::from_bytes(bytes).unwrap();
    assert_eq!(
        verifier
            .verify(
                &RingSha256Hasher.hash_bytes(&fx.statement),
                &fx.leaf,
                &invalid,
                &fx.root,
                &fx.crl,
                fx.at
            )
            .unwrap_err(),
        CredentialFailure::InvalidSignature
    );
}

#[test]
fn raw_material_limits_apply_before_parsing() {
    let verifier = InternalRsaDeclarationVerifier::new();
    assert_eq!(
        verifier
            .inspect_certificate(&vec![0; 16 * 1024 + 1])
            .unwrap_err(),
        CredentialFailure::LimitExceeded
    );
    assert_eq!(
        verifier
            .inspect_trust(&vec![0; 16 * 1024 + 1], b"", 0)
            .unwrap_err(),
        CredentialFailure::LimitExceeded
    );
    assert_eq!(
        verifier
            .inspect_trust(b"", &vec![0; 1024 * 1024 + 1], 0)
            .unwrap_err(),
        CredentialFailure::LimitExceeded
    );
}

#[test]
fn pem_concatenation_private_keys_and_trailing_data_are_rejected() {
    let fx = fixture();
    let verifier = InternalRsaDeclarationVerifier::new();
    for bad in [
        [fx.leaf.as_slice(), &fx.leaf].concat(),
        [fx.leaf.as_slice(), b"trailing"].concat(),
        [fx.leaf_der.as_slice(), &[0]].concat(),
        b"-----BEGIN PRIVATE KEY-----\nYQ==\n-----END PRIVATE KEY-----\n".to_vec(),
    ] {
        assert_eq!(
            verifier.inspect_certificate(&bad).unwrap_err(),
            CredentialFailure::MalformedCertificate
        );
    }
}

#[test]
fn exact_pem_size_limits_preserve_canonical_public_bytes() {
    let fx = fixture();
    let verifier = InternalRsaDeclarationVerifier::new();
    let mut certificate = fx.leaf.clone();
    certificate.resize(16 * 1024, b' ');
    assert_eq!(
        verifier.inspect_certificate(&certificate).unwrap().der,
        fx.leaf_der
    );
    let mut root = fx.root.clone();
    root.resize(16 * 1024, b' ');
    let mut crl = fx.crl.clone();
    crl.resize(1024 * 1024, b' ');
    assert_eq!(
        verifier.inspect_trust(&root, &crl, fx.at).unwrap(),
        verifier.inspect_trust(&fx.root, &fx.crl, fx.at).unwrap()
    );
}
