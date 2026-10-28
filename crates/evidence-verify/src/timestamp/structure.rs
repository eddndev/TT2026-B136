//! Location of the parts of a response or token over the original bytes.
//!
//! Every part is a borrowed slice of the input found with the bounded DER
//! walker, so the signature can be checked over `signedAttrs` exactly as
//! encoded and the message digest over the exact encapsulated content.
//! Decoding the CMS structure with a library would sort and re-encode its
//! sets. Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

use der::asn1::ObjectIdentifier;

use super::{malformed, ID_CT_TST_INFO};
use crate::der_walk::{self, DerError, Tlv};
use crate::report::Finding;

pub(super) const SEQUENCE: u8 = 0x30;
pub(super) const SET: u8 = 0x31;
pub(super) const INTEGER: u8 = 0x02;
pub(super) const OID: u8 = 0x06;
pub(super) const OCTET_STRING: u8 = 0x04;
pub(super) const BIT_STRING: u8 = 0x03;
/// `[0]` constructed, context specific.
pub(super) const CONTEXT_0: u8 = 0xA0;
/// `[1]` constructed, context specific.
pub(super) const CONTEXT_1: u8 = 0xA1;

/// `id-signedData`, the only admitted ContentInfo type of a token.
const ID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");

/// Most algorithm identifiers admitted in `digestAlgorithms`.
const MAX_DIGEST_ALGORITHMS: usize = 8;
/// Most certificates admitted inside a token.
const MAX_CERTIFICATES: usize = 16;
/// Most SignerInfo elements read; RFC 3161 admits exactly one, so more than
/// one is a failed `token.signer_count`.
const MAX_SIGNERS: usize = 8;

/// The parts of a SignedData token that the checks need.
#[derive(Debug)]
pub(super) struct SignedToken<'a> {
    /// Elements of the `digestAlgorithms` SET.
    pub digest_algorithms: Vec<Tlv<'a>>,
    /// Content octets of the encapsulated OCTET STRING: the TSTInfo DER.
    pub econtent: &'a [u8],
    /// Every embedded certificate, in token order.
    pub certificates: Vec<Tlv<'a>>,
    /// The single SignerInfo.
    pub signer: SignerInfo<'a>,
}

/// The fields of the single SignerInfo.
#[derive(Debug)]
pub(super) struct SignerInfo<'a> {
    pub version: u8,
    /// `issuerAndSerialNumber` (a SEQUENCE) or `[0]` subject key identifier.
    pub sid: Tlv<'a>,
    pub digest_algorithm: Tlv<'a>,
    /// The `[0]` IMPLICIT `signedAttrs` element exactly as encoded.
    pub signed_attrs: Tlv<'a>,
    pub signature_algorithm: Tlv<'a>,
    /// Content octets of the signature OCTET STRING.
    pub signature: &'a [u8],
}

/// Return the ContentInfo token of `input`: the token inside a granted
/// `TimeStampResp`, or `input` itself when it is a bare ContentInfo.
///
/// A response whose status is neither granted (0) nor grantedWithMods (1)
/// is a failed `token.not_granted`; a granted response without a token is a
/// failed `token.missing`.
pub(super) fn token_of(input: &[u8]) -> Result<&[u8], Finding> {
    let outer = der_walk::read_exact(input).map_err(der)?;
    expect_tag(&outer, SEQUENCE, "response or token")?;
    let parts = der_walk::children(outer.value, 2).map_err(der)?;
    match parts.first().map(|part| part.tag) {
        Some(OID) => Ok(input),
        Some(SEQUENCE) => {
            check_status(&parts[0])?;
            parts.get(1).map(|token| token.raw).ok_or_else(|| {
                Finding::failed("token.missing", "the granted response carries no token")
            })
        }
        _ => Err(malformed("neither a time-stamp response nor a token")),
    }
}

/// Locate the parts of the ContentInfo `token`.
pub(super) fn signed_token(token: &[u8]) -> Result<SignedToken<'_>, Finding> {
    let content_info = der_walk::read_exact(token).map_err(der)?;
    expect_tag(&content_info, SEQUENCE, "ContentInfo")?;
    let [content_type, explicit] = exactly::<2>(content_info.value, "ContentInfo")?;
    expect_tag(&content_type, OID, "ContentInfo content type")?;
    if super::attributes::oid(content_type)? != ID_SIGNED_DATA {
        return Err(malformed("the token is not CMS SignedData"));
    }
    expect_tag(&explicit, CONTEXT_0, "ContentInfo content")?;
    let [signed_data] = exactly::<1>(explicit.value, "ContentInfo content")?;
    expect_tag(&signed_data, SEQUENCE, "SignedData")?;
    let fields = der_walk::children(signed_data.value, 6).map_err(der)?;
    let field = |at: usize, tag: u8, what: &str| match fields.get(at) {
        Some(tlv) if tlv.tag == tag => Ok(*tlv),
        _ => Err(malformed(format!("SignedData {what}"))),
    };
    if small_uint(&field(0, INTEGER, "version")?)? != 3 {
        return Err(malformed("SignedData version is not 3"));
    }
    let algorithms = field(1, SET, "digestAlgorithms")?;
    let digest_algorithms =
        der_walk::children(algorithms.value, MAX_DIGEST_ALGORITHMS).map_err(der)?;
    let econtent = encapsulated_content(field(2, SEQUENCE, "encapContentInfo")?)?;
    let mut next = 3;
    let mut certificates = Vec::new();
    if fields.get(next).is_some_and(|tlv| tlv.tag == CONTEXT_0) {
        certificates = der_walk::children(fields[next].value, MAX_CERTIFICATES).map_err(der)?;
        next += 1;
    }
    if let Some(choice) = certificates.iter().find(|tlv| tlv.tag != SEQUENCE) {
        return Err(Finding::not_supported(
            "token.certificate_choice",
            format!("certificate choice with tag {:#04x}", choice.tag),
        ));
    }
    if fields.get(next).is_some_and(|tlv| tlv.tag == CONTEXT_1) {
        next += 1;
    }
    let signer_infos = field(next, SET, "signerInfos")?;
    if fields.len() != next + 1 {
        return Err(malformed("SignedData has elements after signerInfos"));
    }
    let signers = der_walk::children(signer_infos.value, MAX_SIGNERS).map_err(|error| {
        if error == DerError::TooManyChildren {
            Finding::failed("token.signer_count", "more than one SignerInfo")
        } else {
            der(error)
        }
    })?;
    let signer = match signers.as_slice() {
        [] => return Err(malformed("the token has no SignerInfo")),
        [only] => signer_info(only)?,
        more => {
            return Err(Finding::failed(
                "token.signer_count",
                format!("{} SignerInfo elements", more.len()),
            ))
        }
    };
    Ok(SignedToken {
        digest_algorithms,
        econtent,
        certificates,
        signer,
    })
}

/// Content octets of an INTEGER that fits in one non-negative octet.
pub(super) fn small_uint(tlv: &Tlv<'_>) -> Result<u8, Finding> {
    match (tlv.tag, tlv.value) {
        (INTEGER, &[value]) if value < 0x80 => Ok(value),
        _ => Err(malformed("expected a small non-negative INTEGER")),
    }
}

pub(super) fn der(error: DerError) -> Finding {
    malformed(error.code())
}

pub(super) fn expect_tag(tlv: &Tlv<'_>, tag: u8, what: &str) -> Result<(), Finding> {
    if tlv.tag == tag {
        Ok(())
    } else {
        Err(malformed(format!("unexpected tag for {what}")))
    }
}

/// The elements of `value`, which must be exactly `N`.
fn exactly<'a, const N: usize>(value: &'a [u8], what: &str) -> Result<[Tlv<'a>; N], Finding> {
    der_walk::children(value, N)
        .map_err(der)?
        .try_into()
        .map_err(|_| malformed(format!("unexpected element count in {what}")))
}

/// PKIStatusInfo: a status INTEGER, then an optional status string and an
/// optional failure information BIT STRING, which are not interpreted.
fn check_status(status_info: &Tlv<'_>) -> Result<(), Finding> {
    let parts = der_walk::children(status_info.value, 3).map_err(der)?;
    let status = parts
        .first()
        .ok_or_else(|| malformed("the response has no status"))?;
    let status = small_uint(status)?;
    let mut rest = &parts[1..];
    if rest.first().is_some_and(|tlv| tlv.tag == SEQUENCE) {
        rest = &rest[1..];
    }
    if rest.first().is_some_and(|tlv| tlv.tag == BIT_STRING) {
        rest = &rest[1..];
    }
    if !rest.is_empty() {
        return Err(malformed("unexpected element in the response status"));
    }
    if status > 1 {
        return Err(Finding::failed(
            "token.not_granted",
            format!("the response status is {status}"),
        ));
    }
    Ok(())
}

/// EncapsulatedContentInfo with type `id-ct-TSTInfo` and present content,
/// returning the content octets of the primitive OCTET STRING.
fn encapsulated_content<'a>(encap: Tlv<'a>) -> Result<&'a [u8], Finding> {
    let parts = der_walk::children(encap.value, 2).map_err(der)?;
    let content_type = parts
        .first()
        .ok_or_else(|| malformed("encapContentInfo has no type"))?;
    expect_tag(content_type, OID, "eContentType")?;
    if super::attributes::oid(*content_type)? != ID_CT_TST_INFO {
        return Err(malformed("the encapsulated content is not TSTInfo"));
    }
    let explicit = parts
        .get(1)
        .ok_or_else(|| malformed("the token has no encapsulated content"))?;
    expect_tag(explicit, CONTEXT_0, "eContent")?;
    let [octets] = exactly::<1>(explicit.value, "eContent")?;
    expect_tag(&octets, OCTET_STRING, "eContent")?;
    Ok(octets.value)
}

/// SignerInfo: version, sid, digestAlgorithm, `[0]` signedAttrs (required
/// for a TSTInfo content), signatureAlgorithm, signature and optional `[1]`
/// unsignedAttrs, which are not interpreted.
fn signer_info<'a>(info: &Tlv<'a>) -> Result<SignerInfo<'a>, Finding> {
    expect_tag(info, SEQUENCE, "SignerInfo")?;
    let fields = der_walk::children(info.value, 7).map_err(der)?;
    if fields.len() == 7 {
        expect_tag(&fields[6], CONTEXT_1, "unsignedAttrs")?;
    } else if fields.len() != 6 {
        return Err(malformed("unexpected element count in SignerInfo"));
    }
    expect_tag(&fields[2], SEQUENCE, "digestAlgorithm")?;
    expect_tag(&fields[3], CONTEXT_0, "signedAttrs")?;
    expect_tag(&fields[4], SEQUENCE, "signatureAlgorithm")?;
    expect_tag(&fields[5], OCTET_STRING, "signature")?;
    Ok(SignerInfo {
        version: small_uint(&fields[0])?,
        sid: fields[1],
        digest_algorithm: fields[2],
        signed_attrs: fields[3],
        signature_algorithm: fields[4],
        signature: fields[5].value,
    })
}
