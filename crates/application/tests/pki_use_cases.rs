//! Use case tests exercising the public-key-infrastructure flows against
//! mocked authority and validator ports.

use application::pki::{
    GenerateCrl, InitializeCa, InspectCertificate, IssueCertificate, RevokeCertificate,
    ValidateCertificate,
};
use application::ApplicationError;
use domain::crypto::certificate::{
    CertificateAuthority, CertificateSummary, CertificateValidation, CertificateValidator,
    IssuedCertificate,
};
use domain::DomainError;
use std::path::PathBuf;

mockall::mock! {
    Authority {}

    impl CertificateAuthority for Authority {
        fn init_ca(&self) -> Result<Vec<u8>, DomainError>;

        fn root_certificate(&self) -> Result<Vec<u8>, DomainError>;

        fn issue(&self, common_name: &str) -> Result<IssuedCertificate, DomainError>;

        fn revoke(&self, serial_hex: &str) -> Result<(), DomainError>;

        fn generate_crl(&self) -> Result<Vec<u8>, DomainError>;
    }
}

mockall::mock! {
    Validator {}

    impl CertificateValidator for Validator {
        fn validate<'a>(
            &self,
            certificate: &[u8],
            issuer: &[u8],
            crl: Option<&'a [u8]>,
            unix_seconds: i64,
        ) -> Result<CertificateValidation, DomainError>;

        fn inspect(&self, certificate: &[u8]) -> Result<CertificateSummary, DomainError>;
    }
}

const ROOT_PEM: &[u8] = b"root certificate pem";
const CERT_PEM: &[u8] = b"issued certificate pem";
const EVAL_TIME: i64 = 1_800_000_000;

fn issued_fixture() -> IssuedCertificate {
    IssuedCertificate {
        certificate_pem: CERT_PEM.to_vec(),
        certificate_path: PathBuf::from("/ca/certs/ana-prueba.crt.pem"),
        private_key_path: PathBuf::from("/ca/private/ana-prueba.key.pem"),
        serial_hex: "1000".to_string(),
    }
}

#[test]
fn initialize_ca_returns_the_root_certificate() {
    let mut authority = MockAuthority::new();
    authority
        .expect_init_ca()
        .times(1)
        .returning(|| Ok(ROOT_PEM.to_vec()));

    let root = InitializeCa::new(authority).execute().unwrap();
    assert_eq!(root, ROOT_PEM);
}

#[test]
fn initialize_ca_propagates_an_authority_failure() {
    let mut authority = MockAuthority::new();
    authority.expect_init_ca().returning(|| {
        Err(DomainError::CertificateAuthorityFailure(
            "script not found".to_string(),
        ))
    });

    let err = InitializeCa::new(authority).execute().unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::CertificateAuthorityFailure(_))
    ));
}

#[test]
fn issue_reports_success_only_after_the_certificate_chains_to_the_root() {
    let mut authority = MockAuthority::new();
    let mut validator = MockValidator::new();

    authority
        .expect_issue()
        .times(1)
        .withf(|common_name| common_name == "Ana Prueba")
        .returning(|_| Ok(issued_fixture()));
    authority
        .expect_root_certificate()
        .times(1)
        .returning(|| Ok(ROOT_PEM.to_vec()));
    validator
        .expect_validate()
        .times(1)
        .withf(|cert, issuer, crl, at| {
            cert == CERT_PEM && issuer == ROOT_PEM && crl.is_none() && *at == EVAL_TIME
        })
        .returning(|_, _, _, _| Ok(CertificateValidation::Valid));

    let issued = IssueCertificate::new(authority, validator)
        .execute("Ana Prueba", EVAL_TIME)
        .unwrap();
    assert_eq!(issued, issued_fixture());
}

#[test]
fn issue_rejects_a_certificate_that_does_not_validate() {
    let mut authority = MockAuthority::new();
    let mut validator = MockValidator::new();

    authority.expect_issue().returning(|_| Ok(issued_fixture()));
    authority
        .expect_root_certificate()
        .returning(|| Ok(ROOT_PEM.to_vec()));
    validator
        .expect_validate()
        .returning(|_, _, _, _| Ok(CertificateValidation::UntrustedIssuer));

    let err = IssueCertificate::new(authority, validator)
        .execute("Ana Prueba", EVAL_TIME)
        .unwrap_err();
    match err {
        ApplicationError::IssuedCertificateInvalid(outcome) => {
            assert_eq!(outcome, "untrusted issuer");
        }
        other => panic!("expected a post-issuance validation error, got {other:?}"),
    }
}

#[test]
fn issue_propagates_an_authority_failure_without_validating() {
    let mut authority = MockAuthority::new();
    let validator = MockValidator::new();

    authority.expect_issue().returning(|_| {
        Err(DomainError::CertificateAuthorityFailure(
            "CA not initialized".to_string(),
        ))
    });

    let err = IssueCertificate::new(authority, validator)
        .execute("Ana Prueba", EVAL_TIME)
        .unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::CertificateAuthorityFailure(_))
    ));
}

#[test]
fn revoke_passes_the_serial_through() {
    let mut authority = MockAuthority::new();
    authority
        .expect_revoke()
        .times(1)
        .withf(|serial| serial == "1000")
        .returning(|_| Ok(()));

    RevokeCertificate::new(authority).execute("1000").unwrap();
}

#[test]
fn generate_crl_returns_the_fresh_list() {
    let mut authority = MockAuthority::new();
    authority
        .expect_generate_crl()
        .times(1)
        .returning(|| Ok(b"crl pem".to_vec()));

    let crl = GenerateCrl::new(authority).execute().unwrap();
    assert_eq!(crl, b"crl pem");
}

#[test]
fn validate_forwards_all_inputs_and_returns_the_outcome() {
    let mut validator = MockValidator::new();
    validator
        .expect_validate()
        .times(1)
        .withf(|cert, issuer, crl, at| {
            cert == CERT_PEM && issuer == ROOT_PEM && crl == &Some(&b"crl pem"[..]) && *at == 42
        })
        .returning(|_, _, _, _| {
            Ok(CertificateValidation::Revoked {
                serial_hex: "1000".to_string(),
            })
        });

    let outcome = ValidateCertificate::new(validator)
        .execute(CERT_PEM, ROOT_PEM, Some(b"crl pem"), 42)
        .unwrap();
    assert_eq!(
        outcome,
        CertificateValidation::Revoked {
            serial_hex: "1000".to_string(),
        }
    );
}

#[test]
fn validate_propagates_a_stale_crl_error() {
    let mut validator = MockValidator::new();
    validator.expect_validate().returning(|_, _, _, _| {
        Err(DomainError::StaleCrl {
            next_update_unix: 41,
        })
    });

    let err = ValidateCertificate::new(validator)
        .execute(CERT_PEM, ROOT_PEM, Some(b"crl pem"), 42)
        .unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::Domain(DomainError::StaleCrl {
            next_update_unix: 41
        })
    ));
}

#[test]
fn inspect_returns_the_summary() {
    let summary = CertificateSummary {
        subject: "CN=Ana Prueba".to_string(),
        issuer: "CN=CA Raiz".to_string(),
        serial_hex: "1000".to_string(),
        not_before_unix: 100,
        not_after_unix: 200,
    };
    let expected = summary.clone();

    let mut validator = MockValidator::new();
    validator
        .expect_inspect()
        .times(1)
        .withf(|cert| cert == CERT_PEM)
        .returning(move |_| Ok(summary.clone()));

    let result = InspectCertificate::new(validator)
        .execute(CERT_PEM)
        .unwrap();
    assert_eq!(result, expected);
}
