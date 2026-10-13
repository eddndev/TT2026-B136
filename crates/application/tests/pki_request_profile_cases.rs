//! Profile selection must cross issuance and validation without substitution.

use super::{
    chain_validator, expect_root, public_certificate, ApplicationError, CertificateRequestProfile,
    CertificateValidation, CredentialFailure, IssueCertificateRequest, MockAuthority, MockProfiles,
    Sequence, CERT_PEM, EVAL_TIME, REQUEST_NAME, REQUEST_PEM,
};

#[test]
fn each_profile_is_forwarded_to_the_authority_and_checked_after_chain_validation() {
    for expected in [
        CertificateRequestProfile::DocumentSigning,
        CertificateRequestProfile::Partner,
    ] {
        let mut sequence = Sequence::new();
        let mut authority = MockAuthority::new();
        authority.expect_issue().never();
        authority
            .expect_issue_request()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(move |name, request, profile| {
                name == REQUEST_NAME && request == REQUEST_PEM && *profile == expected
            })
            .returning(|_, _, _| Ok(public_certificate()));
        expect_root(&mut authority, &mut sequence);
        let validator = chain_validator(Ok(CertificateValidation::Valid), &mut sequence);
        let mut profiles = MockProfiles::new();
        profiles
            .expect_validate_profile()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(move |certificate, profile| certificate == CERT_PEM && *profile == expected)
            .returning(|_, _| Ok(()));

        let result = IssueCertificateRequest::new(authority, validator, profiles)
            .execute(REQUEST_NAME, REQUEST_PEM, expected, EVAL_TIME)
            .unwrap();

        assert_eq!(result, public_certificate());
    }
}

#[test]
fn a_partner_request_rejects_a_valid_chain_with_the_wrong_profile() {
    let mut sequence = Sequence::new();
    let mut authority = MockAuthority::new();
    authority.expect_issue().never();
    authority
        .expect_issue_request()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(|name, request, profile| {
            name == REQUEST_NAME
                && request == REQUEST_PEM
                && *profile == CertificateRequestProfile::Partner
        })
        .returning(|_, _, _| Ok(public_certificate()));
    expect_root(&mut authority, &mut sequence);
    let validator = chain_validator(Ok(CertificateValidation::Valid), &mut sequence);
    let mut profiles = MockProfiles::new();
    profiles
        .expect_validate_profile()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(|certificate, profile| {
            certificate == CERT_PEM && *profile == CertificateRequestProfile::Partner
        })
        .returning(|_, _| Err(CredentialFailure::UnsupportedCertificate));

    let error = IssueCertificateRequest::new(authority, validator, profiles)
        .execute(
            REQUEST_NAME,
            REQUEST_PEM,
            CertificateRequestProfile::Partner,
            EVAL_TIME,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        ApplicationError::IssuedCertificateInvalid(reason)
            if reason == CredentialFailure::UnsupportedCertificate.to_string()
    ));
}
