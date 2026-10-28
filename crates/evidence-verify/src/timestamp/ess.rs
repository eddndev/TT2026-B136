//! ESS signing certificate attributes (RFC 2634 and RFC 5035), which bind
//! the signer certificate into the signed attributes.
//!
//! The `cms` crate does not define these types, so they are defined here.
//! Only the first certificate identifier is checked: it names the signer;
//! later ones, if any, name chain certificates and are not interpreted.

use der::asn1::OctetString;
use der::{DecodeOwned, Encode, Sequence};
use sha1::{Digest, Sha1};
use spki::AlgorithmIdentifierOwned;
use x509_cert::ext::pkix::certpolicy::PolicyInformation;
use x509_cert::ext::pkix::name::{GeneralName, GeneralNames};
use x509_cert::serial_number::SerialNumber;

use super::malformed;
use super::signer::ID_SHA256;
use crate::certificate::{self, Certificate};
use crate::report::Finding;

/// `SigningCertificate ::= SEQUENCE { certs SEQUENCE OF ESSCertID,
/// policies SEQUENCE OF PolicyInformation OPTIONAL }`
#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
pub(crate) struct SigningCertificate {
    pub certs: Vec<EssCertId>,
    #[asn1(optional = "true")]
    pub policies: Option<Vec<PolicyInformation>>,
}

/// `ESSCertID ::= SEQUENCE { certHash OCTET STRING (SHA-1),
/// issuerSerial IssuerSerial OPTIONAL }`
#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
pub(crate) struct EssCertId {
    pub cert_hash: OctetString,
    #[asn1(optional = "true")]
    pub issuer_serial: Option<IssuerSerial>,
}

/// `SigningCertificateV2 ::= SEQUENCE { certs SEQUENCE OF ESSCertIDv2,
/// policies SEQUENCE OF PolicyInformation OPTIONAL }`
#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
pub(crate) struct SigningCertificateV2 {
    pub certs: Vec<EssCertIdV2>,
    #[asn1(optional = "true")]
    pub policies: Option<Vec<PolicyInformation>>,
}

/// `ESSCertIDv2 ::= SEQUENCE { hashAlgorithm AlgorithmIdentifier DEFAULT
/// id-sha256, certHash OCTET STRING, issuerSerial IssuerSerial OPTIONAL }`
#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
pub(crate) struct EssCertIdV2 {
    #[asn1(optional = "true")]
    pub hash_algorithm: Option<AlgorithmIdentifierOwned>,
    pub cert_hash: OctetString,
    #[asn1(optional = "true")]
    pub issuer_serial: Option<IssuerSerial>,
}

/// `IssuerSerial ::= SEQUENCE { issuer GeneralNames,
/// serialNumber CertificateSerialNumber }`
#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
pub(crate) struct IssuerSerial {
    pub issuer: GeneralNames,
    pub serial_number: SerialNumber,
}

/// Check a `SigningCertificate` value: its first identifier must carry the
/// SHA-1 of the signer certificate DER and a matching issuer and serial when
/// present.
pub(super) fn check_v1(value: &[u8], signer: &Certificate) -> Result<(), Finding> {
    let decoded: SigningCertificate = strict(value)?;
    let first = decoded
        .certs
        .first()
        .ok_or_else(|| mismatch("SigningCertificate names no certificate"))?;
    if first.cert_hash.as_bytes() != Sha1::digest(&signer.der).as_slice() {
        return Err(mismatch(
            "the SigningCertificate hash is not the SHA-1 of the signer certificate",
        ));
    }
    check_issuer_serial(first.issuer_serial.as_ref(), signer)
}

/// Check a `SigningCertificateV2` value: its first identifier must carry
/// the SHA-256 of the signer certificate DER (an absent hash algorithm means
/// SHA-256; any other algorithm is not supported) and a matching issuer and
/// serial when present.
pub(super) fn check_v2(value: &[u8], signer: &Certificate) -> Result<(), Finding> {
    let decoded: SigningCertificateV2 = strict(value)?;
    let first = decoded
        .certs
        .first()
        .ok_or_else(|| mismatch("SigningCertificateV2 names no certificate"))?;
    if let Some(algorithm) = &first.hash_algorithm {
        let plain = algorithm
            .parameters
            .as_ref()
            .is_none_or(|parameters| parameters.is_null());
        if algorithm.oid != ID_SHA256 || !plain {
            return Err(Finding::not_supported(
                "token.ess_hash_algorithm",
                format!("ESSCertIDv2 hash algorithm {}", algorithm.oid),
            ));
        }
    }
    if first.cert_hash.as_bytes() != certificate::sha256(&signer.der) {
        return Err(mismatch(
            "the SigningCertificateV2 hash is not the SHA-256 of the signer certificate",
        ));
    }
    check_issuer_serial(first.issuer_serial.as_ref(), signer)
}

/// A present `issuerSerial` must hold exactly one directory name equal to
/// the signer certificate issuer and the signer certificate serial.
fn check_issuer_serial(
    issuer_serial: Option<&IssuerSerial>,
    signer: &Certificate,
) -> Result<(), Finding> {
    let Some(issuer_serial) = issuer_serial else {
        return Ok(());
    };
    let issuer_matches = matches!(
        issuer_serial.issuer.as_slice(),
        [GeneralName::DirectoryName(name)] if *name == signer.issuer
    );
    if !issuer_matches || issuer_serial.serial_number.as_bytes() != signer.serial {
        return Err(mismatch(
            "the ESS issuerSerial does not name the signer certificate",
        ));
    }
    Ok(())
}

fn mismatch(detail: &str) -> Finding {
    Finding::failed("token.ess_mismatch", detail)
}

/// Decode `bytes` and require that they are the canonical DER of the value.
fn strict<T: DecodeOwned + Encode>(bytes: &[u8]) -> Result<T, Finding> {
    let value = T::from_der(bytes).map_err(|_| malformed("invalid ESS attribute"))?;
    if value.to_der().ok().as_deref() != Some(bytes) {
        return Err(malformed("ESS attribute is not in canonical DER"));
    }
    Ok(value)
}
