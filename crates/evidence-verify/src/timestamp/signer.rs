//! Algorithms and certificate of the single SignerInfo.

use cms::cert::IssuerAndSerialNumber;
use der::asn1::ObjectIdentifier;
use der::{Decode, Encode};

use super::malformed;
use super::structure::{der, expect_tag, OID, SEQUENCE};
use crate::certificate::{Certificate, RSA_ENCRYPTION, RSA_SHA256};
use crate::der_walk::{self, Tlv};
use crate::report::Finding;

/// `id-sha256`, the only admitted digest algorithm.
pub(super) const ID_SHA256: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");

/// Identifier octet of the `[0]` IMPLICIT subject key identifier choice.
const SUBJECT_KEY_ID: u8 = 0x80;

/// The SignerInfo `digestAlgorithm` must be `id-sha256` with absent or NULL
/// parameters, else it is not supported, and must be listed in
/// `digestAlgorithms`, else the token is malformed.
pub(super) fn check_digest_algorithm(
    algorithm: Tlv<'_>,
    listed: &[Tlv<'_>],
) -> Result<(), Finding> {
    if !is_sha256(algorithm)? {
        return Err(Finding::not_supported(
            "token.digest_algorithm",
            "the signer digest algorithm is not SHA-256 with absent or NULL parameters",
        ));
    }
    for candidate in listed {
        if is_sha256(*candidate)? {
            return Ok(());
        }
    }
    Err(malformed(
        "the signer digest algorithm is not listed in digestAlgorithms",
    ))
}

/// The signature algorithm must be `rsaEncryption` or
/// `sha256WithRSAEncryption` with absent or NULL parameters.
pub(super) fn check_signature_algorithm(algorithm: Tlv<'_>) -> Result<(), Finding> {
    let (oid, plain) = parse_algorithm(algorithm)?;
    if plain && (oid == RSA_ENCRYPTION || oid == RSA_SHA256) {
        return Ok(());
    }
    Err(Finding::not_supported(
        "token.signature_algorithm",
        format!("signature algorithm {oid}"),
    ))
}

/// Index of the one certificate that `sid` identifies.
///
/// `issuerAndSerialNumber` requires SignerInfo version 1 and matches issuer
/// name and serial; the subject key identifier requires version 3 and
/// matches the certificate subject key identifier. Any other combination is
/// malformed; no match or several matches are failed.
pub(super) fn find_signer(
    version: u8,
    sid: Tlv<'_>,
    certificates: &[Certificate],
) -> Result<usize, Finding> {
    let matches: Vec<usize> = match (sid.tag, version) {
        (SEQUENCE, 1) => {
            let id = IssuerAndSerialNumber::from_der(sid.raw)
                .map_err(|_| malformed("invalid issuerAndSerialNumber"))?;
            if id.to_der().ok().as_deref() != Some(sid.raw) {
                return Err(malformed("issuerAndSerialNumber is not in canonical DER"));
            }
            let serial = id.serial_number.as_bytes();
            indices(certificates, |cert| {
                cert.issuer == id.issuer && cert.serial == serial
            })
        }
        (SUBJECT_KEY_ID, 3) => indices(certificates, |cert| {
            cert.subject_key_id.as_deref() == Some(sid.value)
        }),
        _ => {
            return Err(malformed(
                "the signer identifier does not match the SignerInfo version",
            ))
        }
    };
    match matches.as_slice() {
        [only] => Ok(*only),
        [] => Err(Finding::failed(
            "token.signer_not_found",
            "no embedded certificate matches the signer identifier",
        )),
        _ => Err(Finding::failed(
            "token.signer_ambiguous",
            "several embedded certificates match the signer identifier",
        )),
    }
}

fn indices(certificates: &[Certificate], matches: impl Fn(&Certificate) -> bool) -> Vec<usize> {
    certificates
        .iter()
        .enumerate()
        .filter(|(_, cert)| matches(cert))
        .map(|(at, _)| at)
        .collect()
}

fn is_sha256(algorithm: Tlv<'_>) -> Result<bool, Finding> {
    let (oid, plain) = parse_algorithm(algorithm)?;
    Ok(plain && oid == ID_SHA256)
}

/// An AlgorithmIdentifier as its OID and whether its parameters are absent
/// or NULL.
fn parse_algorithm(algorithm: Tlv<'_>) -> Result<(ObjectIdentifier, bool), Finding> {
    expect_tag(&algorithm, SEQUENCE, "algorithm identifier")?;
    let parts = der_walk::children(algorithm.value, 2).map_err(der)?;
    let oid = parts
        .first()
        .ok_or_else(|| malformed("empty algorithm identifier"))?;
    expect_tag(oid, OID, "algorithm")?;
    let plain = match parts.get(1) {
        None => true,
        Some(parameters) => parameters.raw == [0x05, 0x00],
    };
    Ok((super::attributes::oid(*oid)?, plain))
}
