//! Byte-level helpers shared by the certificate adapters: PEM or DER
//! detection, X.509 parsing with domain error mapping, and conversions
//! from parsed fields to the domain's display representations.

use der::Decode;
use domain::DomainError;
use x509_cert::certificate::Certificate;
use x509_cert::crl::CertificateList;
use x509_cert::serial_number::SerialNumber;
use x509_cert::time::Time;

/// PEM type label of a certificate.
const CERTIFICATE_LABEL: &str = "CERTIFICATE";

/// PEM type label of a certificate revocation list, as OpenSSL writes it.
const CRL_LABEL: &str = "X509 CRL";

/// Parses certificate bytes that may be PEM or DER.
pub(crate) fn parse_certificate(bytes: &[u8]) -> Result<Certificate, DomainError> {
    let der_bytes = to_der(bytes, CERTIFICATE_LABEL).map_err(DomainError::MalformedCertificate)?;
    Certificate::from_der(&der_bytes)
        .map_err(|err| DomainError::MalformedCertificate(err.to_string()))
}

/// Parses revocation-list bytes that may be PEM or DER.
pub(crate) fn parse_crl(bytes: &[u8]) -> Result<CertificateList, DomainError> {
    let der_bytes = to_der(bytes, CRL_LABEL).map_err(DomainError::MalformedCrl)?;
    CertificateList::from_der(&der_bytes).map_err(|err| DomainError::MalformedCrl(err.to_string()))
}

/// Returns DER bytes from input that may be PEM or DER.
///
/// Input starting with a PEM preamble must carry the expected type label
/// and decode cleanly. Anything else is passed through unchanged so the
/// caller's DER parser reports malformed bytes with its own diagnostics.
fn to_der(bytes: &[u8], expected_label: &str) -> Result<Vec<u8>, String> {
    let trimmed = trim_leading_ascii_whitespace(bytes);
    if !trimmed.starts_with(b"-----BEGIN") {
        return Ok(bytes.to_vec());
    }
    let (label, der_bytes) =
        der::pem::decode_vec(trimmed).map_err(|err| format!("invalid pem: {err}"))?;
    if label != expected_label {
        return Err(format!(
            "expected a {expected_label} pem block, found {label}"
        ));
    }
    Ok(der_bytes)
}

fn trim_leading_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

/// Serial bytes as uppercase hex without leading zeros ("0" when all
/// bytes are zero), matching how OpenSSL prints serial numbers.
pub(crate) fn serial_hex(serial: &SerialNumber) -> String {
    let mut hex = String::with_capacity(serial.as_bytes().len() * 2);
    for byte in serial.as_bytes() {
        hex.push_str(&format!("{byte:02X}"));
    }
    let trimmed = hex.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// An X.509 validity time as unix seconds.
pub(crate) fn time_to_unix(time: &Time) -> i64 {
    time.to_unix_duration().as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::DomainError;

    #[test]
    fn garbage_bytes_are_reported_as_a_malformed_certificate() {
        let err = parse_certificate(b"definitely not der").unwrap_err();
        assert!(matches!(err, DomainError::MalformedCertificate(_)));
    }

    #[test]
    fn garbage_bytes_are_reported_as_a_malformed_crl() {
        let err = parse_crl(b"definitely not der").unwrap_err();
        assert!(matches!(err, DomainError::MalformedCrl(_)));
    }

    #[test]
    fn a_pem_block_with_the_wrong_label_is_rejected() {
        let pem = b"-----BEGIN X509 CRL-----\nAAAA\n-----END X509 CRL-----\n";
        let err = parse_certificate(pem).unwrap_err();
        match err {
            DomainError::MalformedCertificate(detail) => {
                assert!(
                    detail.contains("CERTIFICATE") && detail.contains("X509 CRL"),
                    "detail should name both labels, got: {detail}"
                );
            }
            other => panic!("expected a malformed certificate error, got {other:?}"),
        }
    }

    #[test]
    fn a_corrupt_pem_body_is_rejected() {
        let pem = b"-----BEGIN CERTIFICATE-----\n@@@@\n-----END CERTIFICATE-----\n";
        let err = parse_certificate(pem).unwrap_err();
        assert!(matches!(err, DomainError::MalformedCertificate(_)));
    }

    #[test]
    fn serial_hex_strips_the_der_sign_padding_byte() {
        let serial = SerialNumber::new(&[0x00, 0xff]).unwrap();
        assert_eq!(serial_hex(&serial), "FF");
    }

    #[test]
    fn serial_hex_keeps_inner_zeros() {
        let serial = SerialNumber::new(&[0x10, 0x00]).unwrap();
        assert_eq!(serial_hex(&serial), "1000");
    }

    #[test]
    fn serial_hex_of_zero_is_a_single_digit() {
        let serial = SerialNumber::new(&[0x00]).unwrap();
        assert_eq!(serial_hex(&serial), "0");
    }
}
