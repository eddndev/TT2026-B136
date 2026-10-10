//! Public CSR issuance checks at the application port boundary.

use super::{
    ApplicationError, CertificateSummary, CertificateValidation, DomainError,
    IssuedPublicCertificate, MockAuthority, MockValidator, PathBuf, CERT_PEM, EVAL_TIME, ROOT_PEM,
};
use application::pki::IssueCertificateRequest;
use domain::crypto::{
    CredentialCertificate, CredentialCheck, CredentialFailure, CredentialTrustInspection,
    InternalDeclarationVerifier, Sha256Digest, Signature,
};
use mockall::Sequence;

mockall::mock! {
    Declarations {}

    impl InternalDeclarationVerifier for Declarations {
        fn inspect_certificate(
            &self,
            certificate: &[u8],
        ) -> Result<CredentialCertificate, CredentialFailure>;

        fn inspect_trust(
            &self,
            root: &[u8],
            crl: &[u8],
            at: i64,
        ) -> Result<CredentialTrustInspection, CredentialFailure>;

        fn verify(
            &self,
            digest: &Sha256Digest,
            certificate: &[u8],
            signature: &Signature,
            root: &[u8],
            crl: &[u8],
            at: i64,
        ) -> Result<CredentialCheck, CredentialFailure>;
    }
}

const REQUEST_NAME: &str = "Ana Prueba";
const REQUEST_PEM: &[u8] = b"public certificate request pem";

fn public_certificate() -> IssuedPublicCertificate {
    IssuedPublicCertificate {
        certificate_pem: CERT_PEM.to_vec(),
        certificate_path: PathBuf::from("/ca/certs/ana-prueba.crt.pem"),
        serial_hex: "1000".to_string(),
    }
}

fn issued_authority(sequence: &mut Sequence) -> MockAuthority {
    let mut authority = MockAuthority::new();
    authority.expect_issue().never();
    authority
        .expect_issue_request()
        .times(1)
        .in_sequence(sequence)
        .withf(|name, request| name == REQUEST_NAME && request == REQUEST_PEM)
        .returning(|_, _| Ok(public_certificate()));
    authority
}

fn expect_root(authority: &mut MockAuthority, sequence: &mut Sequence) {
    authority
        .expect_root_certificate()
        .times(1)
        .in_sequence(sequence)
        .returning(|| Ok(ROOT_PEM.to_vec()));
}

fn chain_validator(
    outcome: Result<CertificateValidation, DomainError>,
    sequence: &mut Sequence,
) -> MockValidator {
    let mut validator = MockValidator::new();
    validator
        .expect_validate()
        .times(1)
        .in_sequence(sequence)
        .withf(|certificate, root, crl, at| {
            certificate == CERT_PEM && root == ROOT_PEM && crl.is_none() && *at == EVAL_TIME
        })
        .return_once(move |_, _, _, _| outcome);
    validator
}

#[test]
fn request_returns_exact_public_result_after_chain_and_profile_checks() {
    let mut sequence = Sequence::new();
    let mut authority = issued_authority(&mut sequence);
    expect_root(&mut authority, &mut sequence);
    let validator = chain_validator(Ok(CertificateValidation::Valid), &mut sequence);
    let mut declarations = MockDeclarations::new();
    declarations
        .expect_inspect_certificate()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(|certificate| certificate == CERT_PEM)
        .returning(|_| {
            Ok(CredentialCertificate {
                der: b"public certificate der".to_vec(),
                fingerprint: Sha256Digest::from_array([7; 32]),
                summary: CertificateSummary {
                    subject: "CN=Ana Prueba".to_string(),
                    issuer: "CN=CA Raiz".to_string(),
                    serial_hex: "1000".to_string(),
                    not_before_unix: EVAL_TIME - 1,
                    not_after_unix: EVAL_TIME + 1,
                },
            })
        });

    let result = IssueCertificateRequest::new(authority, validator, declarations)
        .execute(REQUEST_NAME, REQUEST_PEM, EVAL_TIME)
        .unwrap();

    assert_eq!(result, public_certificate());
}

#[test]
fn request_rejects_an_untrusted_chain_without_inspecting_the_profile() {
    let mut sequence = Sequence::new();
    let mut authority = issued_authority(&mut sequence);
    expect_root(&mut authority, &mut sequence);
    let validator = chain_validator(Ok(CertificateValidation::UntrustedIssuer), &mut sequence);
    let mut declarations = MockDeclarations::new();
    declarations.expect_inspect_certificate().never();

    let error = IssueCertificateRequest::new(authority, validator, declarations)
        .execute(REQUEST_NAME, REQUEST_PEM, EVAL_TIME)
        .unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::IssuedCertificateInvalid(reason) if reason == "untrusted issuer"
    ));
}

#[test]
fn request_rejects_a_valid_chain_with_an_unsupported_profile() {
    let mut sequence = Sequence::new();
    let mut authority = issued_authority(&mut sequence);
    expect_root(&mut authority, &mut sequence);
    let validator = chain_validator(Ok(CertificateValidation::Valid), &mut sequence);
    let mut declarations = MockDeclarations::new();
    declarations
        .expect_inspect_certificate()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(|certificate| certificate == CERT_PEM)
        .returning(|_| Err(CredentialFailure::UnsupportedCertificate));

    let error = IssueCertificateRequest::new(authority, validator, declarations)
        .execute(REQUEST_NAME, REQUEST_PEM, EVAL_TIME)
        .unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::IssuedCertificateInvalid(reason)
            if reason == CredentialFailure::UnsupportedCertificate.to_string()
    ));
}

#[test]
fn request_propagates_issuance_failure_without_reading_root_or_validating() {
    let mut authority = MockAuthority::new();
    authority.expect_issue().never();
    authority
        .expect_issue_request()
        .times(1)
        .withf(|name, request| name == REQUEST_NAME && request == REQUEST_PEM)
        .returning(|_, _| {
            Err(DomainError::CertificateAuthorityFailure(
                "CSR proof rejected".to_string(),
            ))
        });
    authority.expect_root_certificate().never();
    let mut validator = MockValidator::new();
    validator.expect_validate().never();
    let mut declarations = MockDeclarations::new();
    declarations.expect_inspect_certificate().never();

    let error = IssueCertificateRequest::new(authority, validator, declarations)
        .execute(REQUEST_NAME, REQUEST_PEM, EVAL_TIME)
        .unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::Domain(DomainError::CertificateAuthorityFailure(reason))
            if reason == "CSR proof rejected"
    ));
}

#[test]
fn request_propagates_root_failure_without_validating() {
    let mut sequence = Sequence::new();
    let mut authority = issued_authority(&mut sequence);
    authority
        .expect_root_certificate()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|| {
            Err(DomainError::CertificateAuthorityFailure(
                "root unavailable".to_string(),
            ))
        });
    let mut validator = MockValidator::new();
    validator.expect_validate().never();
    let mut declarations = MockDeclarations::new();
    declarations.expect_inspect_certificate().never();

    let error = IssueCertificateRequest::new(authority, validator, declarations)
        .execute(REQUEST_NAME, REQUEST_PEM, EVAL_TIME)
        .unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::Domain(DomainError::CertificateAuthorityFailure(reason))
            if reason == "root unavailable"
    ));
}

#[test]
fn request_propagates_validator_failure_without_inspecting_the_profile() {
    let mut sequence = Sequence::new();
    let mut authority = issued_authority(&mut sequence);
    expect_root(&mut authority, &mut sequence);
    let validator = chain_validator(
        Err(DomainError::MalformedCertificate("invalid DER".to_string())),
        &mut sequence,
    );
    let mut declarations = MockDeclarations::new();
    declarations.expect_inspect_certificate().never();

    let error = IssueCertificateRequest::new(authority, validator, declarations)
        .execute(REQUEST_NAME, REQUEST_PEM, EVAL_TIME)
        .unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::Domain(DomainError::MalformedCertificate(reason))
            if reason == "invalid DER"
    ));
}
