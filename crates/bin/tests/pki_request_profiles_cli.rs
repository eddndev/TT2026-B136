//! Externally held keys retain distinct document-signing and Owner profiles.

#[path = "pki_request_profile_support/mod.rs"]
mod support;

use std::fs;

use domain::crypto::{CredentialFailure, InternalDeclarationVerifier};
use infrastructure::certificates::{
    InternalRsaDeclarationVerifier, InternalRsaOwnerBindingVerifier,
};
use support::{
    issue, issued_path, pki, public_key, replacement_script, request, scripts, state, success,
};

#[test]
fn default_document_signing_and_explicit_partner_keep_the_external_key_and_exact_profile() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &scripts(), &["init-ca"]));
    let holder = scratch.path().join("holder");
    let csr = request(&holder);
    let holder_before = state(&holder);
    let private_before = state(&ca.join("private"));
    let expected_key = public_key("req", &csr);

    for (name, purpose, partner) in [
        ("default-document", None, false),
        ("owner-access", Some("partner"), true),
        ("explicit-document", Some("document-signing"), false),
    ] {
        let certificate = issued_path(issue(&ca, &scripts(), &csr, name, purpose));
        assert_eq!(
            certificate,
            ca.join("certs").join(format!("{name}.crt.pem"))
        );
        assert_eq!(public_key("x509", &certificate), expected_key);
        let bytes = fs::read(certificate).unwrap();
        let owner = InternalRsaOwnerBindingVerifier::new().inspect_certificate(&bytes);
        let signing = InternalRsaDeclarationVerifier::new().inspect_certificate(&bytes);
        if partner {
            owner.expect("the exact Partner profile is admitted for Owner registration");
            assert_eq!(
                signing.unwrap_err(),
                CredentialFailure::UnsupportedCertificate
            );
        } else {
            signing.expect("the default remains the exact signing-only profile");
            assert_eq!(
                owner.unwrap_err(),
                CredentialFailure::UnsupportedCertificate
            );
        }
        assert_eq!(
            state(&holder),
            holder_before,
            "holder material is unchanged"
        );
        assert_eq!(state(&ca.join("private")), private_before);
        assert!(!ca.join("private").join(format!("{name}.key.pem")).exists());
    }
}

#[test]
fn a_matching_requested_key_cannot_hide_the_wrong_issued_profile() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &scripts(), &["init-ca"]));
    let holder = scratch.path().join("holder");
    let csr = request(&holder);
    let signing = issued_path(issue(&ca, &scripts(), &csr, "document-source", None));
    let partner = issued_path(issue(
        &ca,
        &scripts(),
        &csr,
        "partner-source",
        Some("partner"),
    ));
    let expected_key = public_key("req", &csr);
    let holder_before = state(&holder);
    let private_before = state(&ca.join("private"));

    for (name, requested, replacement) in [
        ("wrong-owner", Some("partner"), &signing),
        ("wrong-default", None, &partner),
        ("wrong-document", Some("document-signing"), &partner),
    ] {
        assert_eq!(public_key("x509", replacement), expected_key);
        let custom = scratch.path().join(name);
        replacement_script(&custom, replacement);
        let output = issue(&ca, &custom, &csr, name, requested);
        assert!(
            !output.status.success(),
            "a wrong profile cannot report success"
        );
        assert!(
            output.stdout.is_empty(),
            "no successful public result is returned"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&CredentialFailure::UnsupportedCertificate.to_string()),
            "expected profile rejection, got {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(ca.join("certs").join(format!("{name}.crt.pem"))).unwrap(),
            fs::read(replacement).unwrap(),
            "the replacement reached post-issuance validation"
        );
        assert_eq!(state(&holder), holder_before);
        assert_eq!(state(&ca.join("private")), private_before);
    }
}

#[test]
fn partner_issuance_still_rejects_a_certificate_for_another_public_key() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &scripts(), &["init-ca"]));
    let original = request(&scratch.path().join("first-holder"));
    let replacement = issued_path(issue(
        &ca,
        &scripts(),
        &original,
        "partner-source",
        Some("partner"),
    ));
    let holder = scratch.path().join("second-holder");
    let requested = request(&holder);
    assert_ne!(
        public_key("x509", &replacement),
        public_key("req", &requested)
    );
    let holder_before = state(&holder);
    let private_before = state(&ca.join("private"));
    let custom = scratch.path().join("custom");
    replacement_script(&custom, &replacement);

    let output = issue(&ca, &custom, &requested, "substituted", Some("partner"));

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("public key does not match the signed request"),
        "expected exact key rejection, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(state(&holder), holder_before);
    assert_eq!(state(&ca.join("private")), private_before);
}

#[test]
fn unsupported_request_purpose_is_rejected_before_reading_material_or_creating_ca_files() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("absent-ca");
    let output = issue(
        &ca,
        &scripts(),
        &scratch.path().join("absent.csr.pem"),
        "unsupported",
        Some("server-auth"),
    );

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid value"),
        "the purpose must be rejected by CLI parsing"
    );
    assert!(!ca.exists());
}
