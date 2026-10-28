//! RFC 3161 time-stamp tokens of the internal authority.
//!
//! [`parse`] admits a `TimeStampResp` or a bare token, checks its structure
//! and algorithms and locates the signer certificate among the embedded
//! ones, before any cryptographic check. The caller then builds the chain
//! of [`Token::signer_certificate`] to a root pinned for time-stamping with
//! [`crate::certificate`], using [`Token::certificates`] only as candidates
//! for path building, never as anchors: a token that does not chain is not
//! supported whatever its other defects. [`Token::verify`] then checks the
//! admitted policy, the CMS signature over the exact signed attribute
//! bytes, the content type, the message digest, the ESS signing
//! certificate and the `TSTInfo` imprint, and returns the facts the other
//! checks need. Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

mod attributes;
mod ess;
mod signer;
mod structure;
#[cfg(test)]
mod tests;
mod tstinfo;

use der::asn1::ObjectIdentifier;

use crate::certificate::{self, Certificate};
use crate::der_walk;
use crate::report::{Finding, Outcome};

/// Largest response or token admitted, in DER bytes.
pub const TOKEN_LIMIT: usize = 64 * 1024;

/// `id-ct-TSTInfo`, the encapsulated content type of every token.
const ID_CT_TST_INFO: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.1.4");
/// `id-contentType`, the signed attribute naming the encapsulated type.
const ID_CONTENT_TYPE: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.3");
/// `id-messageDigest`, the signed attribute holding the content digest.
const ID_MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
/// `id-aa-signingCertificate`, the ESS attribute with a SHA-1 certificate hash.
const ID_SIGNING_CERTIFICATE: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.12");
/// `id-aa-signingCertificateV2`, the ESS attribute with a SHA-256 hash by default.
const ID_SIGNING_CERTIFICATE_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.2.47");

/// Identifier octet of a universal constructed SET OF, which replaces the
/// `[0]` IMPLICIT tag of `signedAttrs` in the signed input (RFC 5652,
/// section 5.4).
const SET_OF: u8 = 0x31;

/// The structure is not a strict DER token of the admitted shape.
const MALFORMED: &str = "token.malformed";

/// What a verified token states, for the checks the caller still applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenFacts {
    /// `genTime` of the token, Unix seconds.
    pub gen_time: i64,
    /// Whole seconds of the token accuracy, with any milliseconds or
    /// microseconds rounded up to the next second; 0 when absent.
    pub accuracy_seconds: i64,
    /// Policy of the token in dotted decimal form.
    pub policy: String,
    /// Content octets of the token serial number.
    pub serial: Vec<u8>,
    /// Content octets of the nonce, when the token carries one. Reported,
    /// never relied on.
    pub nonce: Option<Vec<u8>>,
    /// The `ordering` field of `TSTInfo`. Reported, never relied on.
    pub ordering: bool,
    /// The `tsa` name of `TSTInfo` in a readable form, when present.
    /// Reported, never relied on.
    pub tsa: Option<String>,
    /// The `signingTime` signed attribute in Unix seconds, when present.
    /// Reported, never relied on.
    pub signing_time: Option<i64>,
    /// DER of the embedded certificate that signed the token.
    pub signer_certificate_der: Vec<u8>,
    /// DER of every certificate embedded in the token, in token order.
    pub certificates: Vec<Vec<u8>>,
    /// Whether the ESS `SigningCertificate` attribute is present and matches.
    pub ess_v1: bool,
    /// Whether the ESS `SigningCertificateV2` attribute is present and matches.
    pub ess_v2: bool,
}

/// A token whose structure and algorithms are admitted and whose signer
/// certificate is located, before any cryptographic check. It borrows the
/// input bytes.
#[derive(Debug)]
pub struct Token<'a> {
    parts: structure::SignedToken<'a>,
    certificates: Vec<Certificate>,
    signer_index: usize,
}

/// Admit the response or bare token `tsr` and locate its signer.
///
/// A response must have status granted (0) or grantedWithMods (1) and carry
/// a token. Input over [`TOKEN_LIMIT`], algorithms other than SHA-256 with
/// RSA PKCS#1 v1.5, a certificate choice other than a certificate and a
/// signer key that is not RSA are not supported. Malformed input, a refused
/// status, a missing token, more than one SignerInfo and a signer identifier
/// that matches no embedded certificate or several are failed.
pub fn parse(tsr: &[u8]) -> Result<Token<'_>, Finding> {
    if tsr.len() > TOKEN_LIMIT {
        return Err(Finding::not_supported(
            "token.too_large",
            format!("token of {} bytes", tsr.len()),
        ));
    }
    der_walk::walk(tsr, der_walk::MAX_DEPTH).map_err(|error| malformed(error.code()))?;
    let token = structure::token_of(tsr)?;
    let parts = structure::signed_token(token)?;
    let info = &parts.signer;
    signer::check_digest_algorithm(info.digest_algorithm, &parts.digest_algorithms)?;
    signer::check_signature_algorithm(info.signature_algorithm)?;
    let certificates = parse_certificates(&parts.certificates)?;
    let signer_index = signer::find_signer(info.version, info.sid, &certificates)?;
    if certificates[signer_index].public_key.is_none() {
        return Err(Finding::not_supported(
            "token.signer_key",
            "the signer key is not an RSA key",
        ));
    }
    Ok(Token {
        parts,
        certificates,
        signer_index,
    })
}

/// Verify the response or bare token `tsr` as a time-stamp over the SHA-256
/// digest `covered_sha256`, admitting only the dotted policies listed in
/// `admitted_policies`: [`parse`] followed by [`Token::verify`], for callers
/// that have already decided how the token chains.
pub fn verify(
    tsr: &[u8],
    covered_sha256: &[u8; 32],
    admitted_policies: &[&str],
) -> Result<TokenFacts, Finding> {
    parse(tsr)?.verify(covered_sha256, admitted_policies)
}

impl Token<'_> {
    /// The embedded certificate that the signer identifier names.
    pub fn signer_certificate(&self) -> &Certificate {
        &self.certificates[self.signer_index]
    }

    /// Every embedded certificate, in token order.
    pub fn certificates(&self) -> &[Certificate] {
        &self.certificates
    }

    /// Check the token as a time-stamp over the SHA-256 digest
    /// `covered_sha256`, admitting only the dotted policies listed in
    /// `admitted_policies`.
    ///
    /// A `TSTInfo` version other than 1, fractional seconds in `genTime`, a
    /// critical extension and a policy outside `admitted_policies` are not
    /// supported, and are reported before the cryptographic checks; only a
    /// `TSTInfo` that cannot be read at all is reported first, as malformed.
    /// Malformed content, an invalid signature, a wrong content type,
    /// message digest or ESS attribute and an imprint over another digest
    /// are failed.
    pub fn verify(
        &self,
        covered_sha256: &[u8; 32],
        admitted_policies: &[&str],
    ) -> Result<TokenFacts, Finding> {
        let info = &self.parts.signer;
        let tst = tstinfo::parse(self.parts.econtent)?;
        let policy = tst.policy.to_string();
        if !admitted_policies.contains(&policy.as_str()) {
            return Err(Finding::not_supported(
                "token.policy_not_admitted",
                format!("policy {policy}"),
            ));
        }
        let signer = self.signer_certificate();
        let key = signer.public_key.as_ref().ok_or_else(|| {
            Finding::not_supported("token.signer_key", "the signer key is not an RSA key")
        })?;
        let signed_attributes = attributes::SignedAttributes::parse(info.signed_attrs.value)?;
        let mut signed = info.signed_attrs.raw.to_vec();
        signed[0] = SET_OF;
        if !certificate::verify_rsa_sha256(key, &signed, info.signature) {
            return Err(Finding::failed(
                "token.signature_invalid",
                "the signer key does not verify the signature over the signed attributes",
            ));
        }
        attributes::check_content_type(&signed_attributes)?;
        attributes::check_message_digest(&signed_attributes, self.parts.econtent)?;
        let (ess_v1, ess_v2) = attributes::check_ess(&signed_attributes, signer)?;
        let signing_time = attributes::signing_time(&signed_attributes)?;
        tst.check_imprint(covered_sha256)?;
        Ok(TokenFacts {
            gen_time: tst.gen_time,
            accuracy_seconds: tst.accuracy_seconds,
            policy,
            serial: tst.serial,
            nonce: tst.nonce,
            ordering: tst.ordering,
            tsa: tst.tsa,
            signing_time,
            signer_certificate_der: signer.der.clone(),
            certificates: self
                .certificates
                .iter()
                .map(|cert| cert.der.clone())
                .collect(),
            ess_v1,
            ess_v2,
        })
    }
}

fn malformed(detail: impl Into<String>) -> Finding {
    Finding::failed(MALFORMED, detail)
}

fn parse_certificates(raw: &[der_walk::Tlv<'_>]) -> Result<Vec<Certificate>, Finding> {
    raw.iter()
        .map(|tlv| {
            certificate::parse(tlv.raw).map_err(|finding| match finding.outcome {
                Outcome::Failed => malformed(format!("embedded certificate: {}", finding.detail)),
                _ => finding,
            })
        })
        .collect()
}
