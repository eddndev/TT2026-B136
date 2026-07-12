//! RFC 3161 timestamp token verifier.
//!
//! Verification runs in two stages. The native stage parses the token
//! DER (a full response or a bare token), checks the message imprint
//! against the expected digest, and reads the generation time. The
//! delegated stage hands the CMS signature and certificate chain check
//! to an `openssl ts -verify` subprocess. Rationale:
//! docs/adr/0005-rfc3161-verification-strategy.md.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use cms::content_info::ContentInfo;
use cms::signed_data::SignedData;
use der::asn1::OctetString;
use der::oid::ObjectIdentifier;
use der::Decode;
use domain::crypto::timestamp::{TimestampVerification, TimestampVerifier};
use domain::crypto::Sha256Digest;
use domain::DomainError;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use x509_tsp::{TimeStampResp, TstInfo};

use crate::error::TsaError;

use super::stderr_fragment;

/// Object identifier of the SHA-256 digest algorithm (id-sha256).
const ID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");

/// Distinguishes the scratch files of concurrent verifications within
/// one process; the process id distinguishes across processes.
static VERIFY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// [`TimestampVerifier`] for RFC 3161 tokens: native parsing and imprint
/// comparison, with the cryptographic check delegated to the openssl
/// command line tool.
#[derive(Debug, Clone, Default)]
pub struct Rfc3161Verifier;

impl Rfc3161Verifier {
    /// Creates the verifier.
    pub fn new() -> Self {
        Self
    }
}

impl TimestampVerifier for Rfc3161Verifier {
    fn verify(
        &self,
        token: &[u8],
        expected: &Sha256Digest,
        trust_anchor_pem: &[u8],
    ) -> Result<TimestampVerification, DomainError> {
        let parsed = match parse_token(token) {
            Ok(parsed) => parsed,
            Err(detail) => return Ok(TimestampVerification::MalformedToken(detail)),
        };
        let imprint = &parsed.tst_info.message_imprint;
        if imprint.hash_algorithm.oid != ID_SHA256
            || imprint.hashed_message.as_bytes() != expected.as_bytes()
        {
            return Ok(TimestampVerification::ImprintMismatch);
        }
        let generated_at = match generation_time_rfc3339(&parsed.tst_info) {
            Ok(text) => text,
            Err(detail) => return Ok(TimestampVerification::MalformedToken(detail)),
        };
        match check_with_openssl(token, expected, trust_anchor_pem, parsed.bare_token)? {
            None => Ok(TimestampVerification::Valid { generated_at }),
            Some(diagnostic) => Ok(TimestampVerification::UntrustedToken(diagnostic)),
        }
    }
}

/// Generation data read natively from a token, for display without a
/// trust decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampTokenInfo {
    /// Generation time asserted by the authority, as RFC 3339 text.
    pub generated_at: String,
    /// Lower-case hex of the SHA-256 imprint the token covers.
    pub digest_hex: String,
}

/// Reads the imprint and generation time out of a token without any
/// signature check.
///
/// # Errors
///
/// Returns [`TsaError::InvalidToken`] when the bytes do not decode as a
/// token or the imprint is not a SHA-256 digest.
pub fn token_info(token: &[u8]) -> Result<TimestampTokenInfo, TsaError> {
    let parsed = parse_token(token).map_err(TsaError::InvalidToken)?;
    let imprint = &parsed.tst_info.message_imprint;
    if imprint.hash_algorithm.oid != ID_SHA256 {
        return Err(TsaError::InvalidToken(format!(
            "the imprint algorithm is not sha-256: {}",
            imprint.hash_algorithm.oid
        )));
    }
    let digest = Sha256Digest::from_bytes(imprint.hashed_message.as_bytes())
        .map_err(|err| TsaError::InvalidToken(err.to_string()))?;
    let generated_at = generation_time_rfc3339(&parsed.tst_info).map_err(TsaError::InvalidToken)?;
    Ok(TimestampTokenInfo {
        generated_at,
        digest_hex: digest.to_hex(),
    })
}

/// Result of the native parsing stage.
struct ParsedToken {
    tst_info: TstInfo,
    /// True when the input was a bare CMS token rather than a full
    /// response; `openssl ts -verify` needs `-token_in` for that form.
    bare_token: bool,
}

fn parse_token(token: &[u8]) -> Result<ParsedToken, String> {
    // A response wraps a status and the token; try it first because it
    // is the form authorities put on the wire.
    if let Ok(response) = TimeStampResp::from_der(token) {
        // PKIStatus granted(0) and grantedWithMods(1) are the only
        // states under which a response carries a token.
        let status = response.status.status as u8;
        if status > 1 {
            return Err(format!("the response denies the request (status {status})"));
        }
        let content = response
            .time_stamp_token
            .ok_or_else(|| "the granted response carries no token".to_string())?;
        return Ok(ParsedToken {
            tst_info: tst_info_of(&content)?,
            bare_token: false,
        });
    }
    let content = ContentInfo::from_der(token)
        .map_err(|err| format!("neither a timestamp response nor a timestamp token: {err}"))?;
    Ok(ParsedToken {
        tst_info: tst_info_of(&content)?,
        bare_token: true,
    })
}

fn tst_info_of(content: &ContentInfo) -> Result<TstInfo, String> {
    let signed: SignedData = content
        .content
        .decode_as()
        .map_err(|err| format!("the token is not CMS signed data: {err}"))?;
    let econtent = signed
        .encap_content_info
        .econtent
        .ok_or_else(|| "the token carries no TSTInfo content".to_string())?;
    let octets: OctetString = econtent
        .decode_as()
        .map_err(|err| format!("the TSTInfo wrapper cannot be decoded: {err}"))?;
    TstInfo::from_der(octets.as_bytes())
        .map_err(|err| format!("the TSTInfo cannot be decoded: {err}"))
}

fn generation_time_rfc3339(tst_info: &TstInfo) -> Result<String, String> {
    let seconds = i64::try_from(tst_info.gen_time.to_unix_duration().as_secs())
        .map_err(|_| "the generation time exceeds the representable range".to_string())?;
    let moment = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|err| format!("the generation time is not representable: {err}"))?;
    moment
        .format(&Rfc3339)
        .map_err(|err| format!("the generation time cannot be rendered: {err}"))
}

/// Runs `openssl ts -verify` over the token and the trust anchor.
///
/// Returns `None` when openssl accepts the token and `Some(diagnostic)`
/// when it refuses it. The error path is reserved for not being able to
/// run the check at all.
fn check_with_openssl(
    token: &[u8],
    expected: &Sha256Digest,
    trust_anchor_pem: &[u8],
    bare_token: bool,
) -> Result<Option<String>, DomainError> {
    let base = std::env::temp_dir().join(format!(
        "rfc3161-verify-{}-{}",
        std::process::id(),
        VERIFY_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let token_path = path_with_suffix(&base, ".token.der");
    let anchor_path = path_with_suffix(&base, ".anchor.pem");
    fs::write(&token_path, token).map_err(|err| backend(format!("cannot stage token: {err}")))?;
    let staged = fs::write(&anchor_path, trust_anchor_pem)
        .map_err(|err| backend(format!("cannot stage trust anchor: {err}")));
    let result = staged.and_then(|()| {
        let mut command = Command::new("openssl");
        command
            .args(["ts", "-verify", "-digest", &expected.to_hex(), "-in"])
            .arg(&token_path);
        if bare_token {
            command.arg("-token_in");
        }
        command.arg("-CAfile").arg(&anchor_path);
        let output = command
            .output()
            .map_err(|err| backend(format!("cannot run openssl ts -verify: {err}")))?;
        if output.status.success() {
            Ok(None)
        } else {
            Ok(Some(stderr_fragment(&output.stderr)))
        }
    });
    let _ = fs::remove_file(&token_path);
    let _ = fs::remove_file(&anchor_path);
    result
}

fn path_with_suffix(base: &std::path::Path, suffix: &str) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn backend(message: String) -> DomainError {
    DomainError::CryptoBackendFailure(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_a_malformed_token() {
        let digest = Sha256Digest::from_array([7u8; 32]);
        let outcome = Rfc3161Verifier::new().verify(&[], &digest, b"").unwrap();
        assert!(matches!(outcome, TimestampVerification::MalformedToken(_)));
    }

    #[test]
    fn truncated_der_is_a_malformed_token_with_a_diagnosis() {
        let digest = Sha256Digest::from_array([7u8; 32]);
        let outcome = Rfc3161Verifier::new()
            .verify(&[0x30, 0x82, 0xff], &digest, b"")
            .unwrap();
        match outcome {
            TimestampVerification::MalformedToken(detail) => {
                assert!(!detail.is_empty(), "the decoder diagnosis must be kept");
            }
            other => panic!("expected a malformed outcome, got {other:?}"),
        }
    }

    #[test]
    fn token_info_rejects_undecodable_bytes() {
        let err = token_info(&[0x01, 0x02]).unwrap_err();
        assert!(matches!(err, TsaError::InvalidToken(_)));
    }
}
