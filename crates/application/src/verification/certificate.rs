//! Certificate component of the verification report.
//!
//! The verdict judges the signer certificate at the signing time: the
//! generation time of a verified timestamp token, or an instant the caller
//! recorded with the signature evidence once that token verifies. The
//! supplied revocation list is the one captured with the signature, so it
//! speaks for that moment. A serial named in the list fails the component
//! whatever its revocation date, so the wording states that the list names
//! it instead of dating the revocation. The status at the evaluation time
//! is appended as an informative note and never changes the outcome.
//! Without a verified token there is no trusted signing time and the
//! certificate is judged at the evaluation time. A revocation list past its
//! next update at the judged instant is an outcome inside the report, never
//! an error. Rationale: docs/adr/0019-immutable-document-versions.md.

use domain::crypto::certificate::{CertificateValidation, CertificateValidator};
use domain::crypto::timestamp::TimestampVerification;
use domain::DomainError;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::{ComponentReport, VerifyDocumentRequest};
use crate::error::ApplicationError;

/// Trusted instant at which the certificate is judged, with its origin.
struct SigningTime {
    unix: i64,
    source: &'static str,
}

/// Resolves the signing time from the token outcome.
///
/// Only a verified token establishes a trusted signing time; `None` means
/// there is none. A recorded instant replaces the token generation time
/// once the token verifies. A verified token whose generation time cannot
/// be read yields `Some(Err(text))` with that unreadable text.
fn signing_time(
    outcome: Option<&TimestampVerification>,
    recorded_unix: Option<i64>,
) -> Option<Result<SigningTime, String>> {
    let Some(TimestampVerification::Valid { generated_at }) = outcome else {
        return None;
    };
    if let Some(unix) = recorded_unix {
        return Some(Ok(SigningTime {
            unix,
            source: "time recorded with the signature evidence, used once the \
                     timestamp token verifies",
        }));
    }
    Some(
        OffsetDateTime::parse(generated_at, &Rfc3339)
            .map(|moment| SigningTime {
                unix: moment.unix_timestamp(),
                source: "generation time of the verified timestamp token",
            })
            .map_err(|_| generated_at.clone()),
    )
}

/// Builds the certificate component for one verification run.
///
/// # Errors
///
/// Unparseable certificate or revocation list bytes, a revocation list not
/// signed by the issuer, and backend failures of the validator remain
/// errors. A revocation list past its next update is reported inside the
/// component.
pub(super) fn certificate_component<C>(
    validator: &C,
    request: &VerifyDocumentRequest<'_>,
    token: Option<&TimestampVerification>,
) -> Result<ComponentReport, ApplicationError>
where
    C: CertificateValidator + ?Sized,
{
    let validate = |crl: Option<&[u8]>, at: i64| {
        validator.validate(
            request.signer_certificate_pem,
            request.issuer_certificate_pem,
            crl,
            at,
        )
    };
    let crl_supplied = request.crl_pem.is_some();
    let evaluation = request.evaluation_unix;
    let signing = match signing_time(token, request.recorded_signing_unix) {
        None => return at_evaluation_time(validate(request.crl_pem, evaluation), request),
        Some(Err(unreadable)) => {
            return Ok(ComponentReport::failed(format!(
                "status at the signing time not established: the generation time \
                 {unreadable:?} of the verified timestamp token cannot be read"
            )));
        }
        Some(Ok(signing)) => signing,
    };

    let instant = format!("{} ({})", rfc3339(signing.unix), signing.source);
    let mut report = match validate(request.crl_pem, signing.unix) {
        Ok(status) => judged(&instant, &status, crl_supplied),
        Err(DomainError::StaleCrl { next_update_unix }) => ComponentReport::failed(format!(
            "status at the signing time {instant} cannot be established: the supplied \
             revocation list was due for its next update at {}, before that time",
            rfc3339(next_update_unix)
        )),
        Err(other) => return Err(other.into()),
    };
    if signing.unix != evaluation {
        let current = match validate(request.crl_pem, evaluation) {
            Ok(status) => describe(&status, crl_supplied),
            Err(DomainError::StaleCrl { next_update_unix }) => {
                let window = match validate(None, evaluation) {
                    Ok(status) => format!("{status} without a revocation check"),
                    Err(err) => format!("not determined ({err})"),
                };
                format!(
                    "{window}; revocation not determined because the supplied revocation \
                     list was due for its next update at {}",
                    rfc3339(next_update_unix)
                )
            }
            Err(err) => format!("not determined ({err})"),
        };
        report.detail.push_str(&format!(
            "; status at the evaluation time {}, informative only and not part of \
             the verdict: {current}",
            rfc3339(evaluation)
        ));
    }
    Ok(report)
}

/// Renders the outcome at the signing time `instant`.
///
/// The validator reports a serial named in the supplied list as revoked
/// without comparing its revocation date with the judged instant. With the
/// list captured at signing every such entry predates the signature; with
/// a later list it may not, so the text states only what the list says.
fn judged(instant: &str, status: &CertificateValidation, crl_supplied: bool) -> ComponentReport {
    match status {
        CertificateValidation::Valid => ComponentReport::passed(format!(
            "status at the signing time {instant}: {}",
            describe(status, crl_supplied)
        )),
        CertificateValidation::Revoked { .. } => ComponentReport::failed(format!(
            "certificate checked for the signing time {instant}: the supplied \
             revocation list names the certificate as {status}; the revocation date \
             is not compared with the signing time"
        )),
        other => ComponentReport::failed(format!("status at the signing time {instant}: {other}")),
    }
}

/// Renders the outcome without a trusted signing time.
///
/// The status speaks for the evaluation time only: a certificate that is
/// expired or revoked then may still have been valid when the signature
/// was made, so the wording states the status and leaves the legal
/// reading to the operator.
fn at_evaluation_time(
    outcome: Result<CertificateValidation, DomainError>,
    request: &VerifyDocumentRequest<'_>,
) -> Result<ComponentReport, ApplicationError> {
    let instant = format!(
        "status at the evaluation time {} (no verified timestamp token establishes \
         a signing time)",
        rfc3339(request.evaluation_unix)
    );
    let status = match outcome {
        Ok(status) => status,
        Err(DomainError::StaleCrl { next_update_unix }) => {
            return Ok(ComponentReport::skipped(format!(
                "certificate status not determined: no verified timestamp token \
                 establishes a signing time, and the supplied revocation list was due \
                 for its next update at {}, before the evaluation time {}",
                rfc3339(next_update_unix),
                rfc3339(request.evaluation_unix)
            )));
        }
        Err(other) => return Err(other.into()),
    };
    let text = format!(
        "{instant}: {}",
        describe(&status, request.crl_pem.is_some())
    );
    Ok(match status {
        CertificateValidation::Valid => ComponentReport::passed(text),
        CertificateValidation::Expired | CertificateValidation::Revoked { .. } => {
            ComponentReport::failed(format!(
                "{text}; this status speaks for the evaluation time only and does not \
                 by itself invalidate a signature made while the certificate was valid"
            ))
        }
        CertificateValidation::NotYetValid | CertificateValidation::UntrustedIssuer => {
            ComponentReport::failed(text)
        }
    })
}

/// Names a status, stating for a valid one whether revocation was checked.
fn describe(status: &CertificateValidation, crl_supplied: bool) -> String {
    match status {
        CertificateValidation::Valid if crl_supplied => {
            "valid (revocation checked against the supplied list)".to_string()
        }
        CertificateValidation::Valid => {
            "valid (revocation not checked: no revocation list supplied)".to_string()
        }
        other => other.to_string(),
    }
}

/// Renders a Unix time as RFC 3339, or as raw seconds when out of range.
fn rfc3339(unix: i64) -> String {
    OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|moment| moment.format(&Rfc3339).ok())
        .unwrap_or_else(|| format!("unix {unix}"))
}
