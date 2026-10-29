//! Use case: integral verification of a signed document.
//!
//! One pass over the document produces a four-component report: document
//! integrity against the digest a timestamp token attests, signature
//! validity under the signer certificate, certificate status at the
//! signing time a verified timestamp token establishes, and the timestamp
//! token's own validity. Every component carries an outcome and its cause;
//! components whose evidence was not supplied are reported as skipped with
//! the reason, never silently omitted.

mod certificate;

use std::fmt;
use std::io::Read;

use domain::crypto::certificate::CertificateValidator;
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
    /// Certificate status at the signing time a verified timestamp token
    /// establishes, or at the evaluation time without one. With a signing
    /// time, the status at the evaluation time is only an informative note
    /// in the detail.
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
    /// Unix time of the verification run. It judges the certificate only
    /// when no verified timestamp token establishes a signing time;
    /// otherwise it only feeds the informative current-status note.
    pub evaluation_unix: i64,
    /// Signing instant recorded by verified evidence other than the token,
    /// such as the acceptance time of an individual signature. It replaces
    /// the token generation time only when the token itself verifies;
    /// `None` judges the certificate at the token generation time.
    pub recorded_signing_unix: Option<i64>,
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
    /// list bytes, a revocation list not signed by the issuer, or a backend
    /// failure of one of the ports. A revocation list past its next update
    /// is an outcome of the certificate component.
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

    let token = match request.timestamp {
        None => None,
        Some(evidence) => {
            Some(timestamp_verifier.verify(evidence.token, &digest, evidence.trust_anchor_pem)?)
        }
    };
    let certificate =
        certificate::certificate_component(certificate_validator, request, token.as_ref())?;

    let (integrity, timestamp) = match &token {
        None => (
            ComponentReport::skipped(
                "digest recomputed from the document, but without a timestamp \
                 token there is no independent reference to compare it against",
            ),
            ComponentReport::skipped("no timestamp token supplied"),
        ),
        Some(outcome) => token_components(outcome),
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
