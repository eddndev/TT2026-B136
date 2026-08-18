//! Use case: integral verification of a signed document.
//!
//! One pass over the document produces a four-component report: document
//! integrity against the digest a timestamp token attests, signature
//! validity under the signer certificate, certificate status at an
//! explicit evaluation time, and the timestamp token's own validity.
//! Every component carries an outcome and its cause; components whose
//! evidence was not supplied are reported as skipped with the reason,
//! never silently omitted.

use std::fmt;
use std::io::Read;

use domain::crypto::certificate::{CertificateValidation, CertificateValidator};
use domain::crypto::timestamp::{TimestampVerification, TimestampVerifier};
use domain::crypto::{DocumentHasher, Signature, SignatureVerification, SignatureVerifier};

use crate::error::ApplicationError;

/// Outcome of one verification component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStatus {
    /// The component was exercised and its check held.
    Passed,
    /// The component was exercised and its check did not hold.
    Failed,
    /// The component was not exercised; the detail carries the reason.
    Skipped,
}

impl fmt::Display for ComponentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => f.write_str("passed"),
            Self::Failed => f.write_str("failed"),
            Self::Skipped => f.write_str("skipped"),
        }
    }
}

/// One verification component: its outcome and the cause behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentReport {
    pub status: ComponentStatus,
    /// Human-readable cause: what was checked and why it passed, failed,
    /// or was skipped.
    pub detail: String,
}

impl ComponentReport {
    fn passed(detail: impl Into<String>) -> Self {
        Self {
            status: ComponentStatus::Passed,
            detail: detail.into(),
        }
    }

    fn failed(detail: impl Into<String>) -> Self {
        Self {
            status: ComponentStatus::Failed,
            detail: detail.into(),
        }
    }

    fn skipped(detail: impl Into<String>) -> Self {
        Self {
            status: ComponentStatus::Skipped,
            detail: detail.into(),
        }
    }
}

/// Overall verdict of a verification run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every exercised component passed; skipped components are reported
    /// but do not count against the verdict.
    Valid,
    /// At least one exercised component failed.
    NotValid,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => f.write_str("valid"),
            Self::NotValid => f.write_str("not valid"),
        }
    }
}

/// The full verification report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    /// Lower-case hex of the SHA-256 digest recomputed from the document.
    pub document_digest_hex: String,
    /// Document integrity against the digest the timestamp token attests.
    pub integrity: ComponentReport,
    /// Signature validity under the signer certificate.
    pub signature: ComponentReport,
    /// Certificate status at the evaluation time.
    pub certificate: ComponentReport,
    /// Validity of the timestamp token under its trust anchor.
    pub timestamp: ComponentReport,
    /// Overall verdict derived from the component outcomes.
    pub verdict: Verdict,
}

/// A timestamp token together with the trust anchor to check it against.
#[derive(Debug, Clone, Copy)]
pub struct TimestampEvidence<'a> {
    /// DER-encoded timestamp response or bare token.
    pub token: &'a [u8],
    /// PEM certificate the token's chain must lead to.
    pub trust_anchor_pem: &'a [u8],
}

/// Evidence and parameters for one verification run. The document itself
/// arrives separately as a stream.
#[derive(Debug, Clone, Copy)]
pub struct VerifyDocumentRequest<'a> {
    /// Detached signature over the document digest.
    pub signature: &'a Signature,
    /// PEM certificate of the signer.
    pub signer_certificate_pem: &'a [u8],
    /// PEM certificate of the authority that issued the signer's.
    pub issuer_certificate_pem: &'a [u8],
    /// Revocation list; `None` skips revocation checking and is reported.
    pub crl_pem: Option<&'a [u8]>,
    /// Timestamp token and anchor; `None` skips the token components and
    /// is reported.
    pub timestamp: Option<TimestampEvidence<'a>>,
    /// Unix time at which the certificate status is evaluated.
    pub evaluation_unix: i64,
}

/// Composes the hashing, signature, certificate, and timestamp ports into
/// one report over a document stream.
pub struct VerifyDocument<H, S, C, T> {
    hasher: H,
    signature_verifier: S,
    certificate_validator: C,
    timestamp_verifier: T,
}

impl<H, S, C, T> VerifyDocument<H, S, C, T>
where
    H: DocumentHasher,
    S: SignatureVerifier,
    C: CertificateValidator,
    T: TimestampVerifier,
{
    /// Builds the use case over the four verification ports.
    pub fn new(
        hasher: H,
        signature_verifier: S,
        certificate_validator: C,
        timestamp_verifier: T,
    ) -> Self {
        Self {
            hasher,
            signature_verifier,
            certificate_validator,
            timestamp_verifier,
        }
    }

    /// Digests `reader` to the end and builds the four-component report.
    ///
    /// # Errors
    ///
    /// Component rejections are outcomes inside the report, never errors.
    /// The error path is reserved for evidence that cannot be assessed at
    /// all: an unreadable stream, unparseable certificate or revocation
    /// list bytes, an untrusted or stale revocation list, or a backend
    /// failure of one of the ports.
    pub fn execute(
        &self,
        reader: &mut dyn Read,
        request: &VerifyDocumentRequest<'_>,
    ) -> Result<VerificationReport, ApplicationError> {
        verify_with_ports(
            &self.hasher,
            &self.signature_verifier,
            &self.certificate_validator,
            &self.timestamp_verifier,
            reader,
            request,
        )
    }
}

/// Verifies one document through borrowed cryptographic ports.
pub fn verify_with_ports<H, S, C, T>(
    hasher: &H,
    signature_verifier: &S,
    certificate_validator: &C,
    timestamp_verifier: &T,
    reader: &mut dyn Read,
    request: &VerifyDocumentRequest<'_>,
) -> Result<VerificationReport, ApplicationError>
where
    H: DocumentHasher + ?Sized,
    S: SignatureVerifier + ?Sized,
    C: CertificateValidator + ?Sized,
    T: TimestampVerifier + ?Sized,
{
    let digest = hasher.hash_stream(reader)?;

    let signature = match signature_verifier.verify(
        &digest,
        request.signature,
        request.signer_certificate_pem,
    )? {
        SignatureVerification::Valid => ComponentReport::passed(
            "signature verifies over the document digest with the signer certificate",
        ),
        SignatureVerification::Invalid(cause) => {
            ComponentReport::failed(format!("signature rejected: {cause}"))
        }
    };

    let status = certificate_validator.validate(
        request.signer_certificate_pem,
        request.issuer_certificate_pem,
        request.crl_pem,
        request.evaluation_unix,
    )?;
    let certificate = certificate_component(&status, request.crl_pem.is_some());

    let (integrity, timestamp) = match request.timestamp {
        None => (
            ComponentReport::skipped(
                "digest recomputed from the document, but without a timestamp \
                 token there is no independent reference to compare it against",
            ),
            ComponentReport::skipped("no timestamp token supplied"),
        ),
        Some(evidence) => {
            let outcome =
                timestamp_verifier.verify(evidence.token, &digest, evidence.trust_anchor_pem)?;
            token_components(&outcome)
        }
    };

    let components = [&integrity, &signature, &certificate, &timestamp];
    let verdict = if components
        .iter()
        .any(|component| component.status == ComponentStatus::Failed)
    {
        Verdict::NotValid
    } else {
        Verdict::Valid
    };

    Ok(VerificationReport {
        document_digest_hex: digest.to_hex(),
        integrity,
        signature,
        certificate,
        timestamp,
        verdict,
    })
}

/// Renders the certificate component from the validation outcome.
///
/// The status speaks for the evaluation time only: a certificate that is
/// expired or revoked now may still have been valid when the signature
/// was made, so the wording states the status and leaves the legal
/// reading to the operator.
fn certificate_component(status: &CertificateValidation, crl_supplied: bool) -> ComponentReport {
    match status {
        CertificateValidation::Valid => {
            let revocation = if crl_supplied {
                "revocation checked against the supplied list"
            } else {
                "revocation not checked: no revocation list supplied"
            };
            ComponentReport::passed(format!(
                "status at the evaluation time: valid ({revocation})"
            ))
        }
        CertificateValidation::Expired | CertificateValidation::Revoked { .. } => {
            ComponentReport::failed(format!(
                "status at the evaluation time: {status}; this status speaks for the \
                 evaluation time only and does not by itself invalidate a signature \
                 made while the certificate was valid"
            ))
        }
        CertificateValidation::NotYetValid | CertificateValidation::UntrustedIssuer => {
            ComponentReport::failed(format!("status at the evaluation time: {status}"))
        }
    }
}

/// Renders the integrity and timestamp components from one token outcome.
///
/// The token verifier reports exactly one outcome and checks the imprint
/// before the trust chain, so an untrusted token still confirms that the
/// recomputed digest matches the attested one.
fn token_components(outcome: &TimestampVerification) -> (ComponentReport, ComponentReport) {
    match outcome {
        TimestampVerification::Valid { generated_at } => (
            ComponentReport::passed(
                "recomputed digest matches the digest the timestamp token attests",
            ),
            ComponentReport::passed(format!(
                "token accepted under the trust anchor; generated at {generated_at}"
            )),
        ),
        TimestampVerification::ImprintMismatch => (
            ComponentReport::failed(
                "recomputed digest differs from the digest the timestamp token attests",
            ),
            ComponentReport::failed("token does not cover the recomputed document digest"),
        ),
        TimestampVerification::MalformedToken(detail) => (
            ComponentReport::skipped(
                "the timestamp token cannot be decoded, so it offers no reference \
                 digest to compare the recomputed digest against",
            ),
            ComponentReport::failed(format!("token cannot be decoded: {detail}")),
        ),
        TimestampVerification::UntrustedToken(detail) => (
            ComponentReport::passed(
                "recomputed digest matches the digest the timestamp token attests",
            ),
            ComponentReport::failed(format!("token rejected under the trust anchor: {detail}")),
        ),
    }
}
