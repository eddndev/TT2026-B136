use std::collections::HashSet;

use der::oid::AssociatedOid;
use der::Decode;
use domain::crypto::{CredentialFailure, CredentialTrustInspection};
use x509_cert::crl::CertificateList;
use x509_cert::ext::pkix::{AuthorityKeyIdentifier, CrlNumber};
use x509_cert::serial_number::SerialNumber;
use x509_cert::Version;

use super::material::{
    der_bytes, digest, encoded, extension, inventory, signature_matches, CRL_LIMIT, RSA_SHA256,
};
use super::profile::ParsedCertificate;
use crate::certificates::parse::time_to_unix;

pub(super) struct ParsedCrl {
    crl: CertificateList,
    der: Vec<u8>,
    number: u64,
    this_update: i64,
    next_update: i64,
}

impl ParsedCrl {
    pub fn revoked(&self, serial: &SerialNumber) -> bool {
        self.crl
            .tbs_cert_list
            .revoked_certificates
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|entry| entry.serial_number == *serial)
    }

    pub fn trust(&self, root: &ParsedCertificate) -> CredentialTrustInspection {
        CredentialTrustInspection {
            root_der: root.der.clone(),
            crl_der: self.der.clone(),
            root_fingerprint: digest(&root.der),
            crl_digest: digest(&self.der),
            crl_number: self.number,
            crl_this_update: self.this_update,
            crl_next_update: self.next_update,
            valid_from: self.this_update.max(root.not_before),
            valid_until: self.next_update.min(root.not_after),
        }
    }
}

pub(super) fn inspect(
    input: &[u8],
    root: &ParsedCertificate,
    at: i64,
) -> Result<ParsedCrl, CredentialFailure> {
    let malformed = CredentialFailure::MalformedCrl;
    let unsupported = CredentialFailure::UnsupportedCrl;
    let der = der_bytes(input, CRL_LIMIT, "X509 CRL", malformed)?;
    let crl = CertificateList::from_der(&der).map_err(|_| malformed)?;
    let tbs = &crl.tbs_cert_list;
    if tbs.version != Version::V2
        || tbs.signature != crl.signature_algorithm
        || tbs.signature.oid != RSA_SHA256
        || !tbs
            .signature
            .parameters
            .as_ref()
            .is_some_and(|v| v.is_null())
        || crl.signature.as_bytes().is_none()
        || crl.signature.raw_bytes().len() != 384
    {
        return Err(unsupported);
    }
    let extensions = tbs.crl_extensions.as_deref().ok_or(unsupported)?;
    inventory(
        extensions,
        &[AuthorityKeyIdentifier::OID, CrlNumber::OID],
        unsupported,
    )?;
    let aki: AuthorityKeyIdentifier = extension(extensions, false, unsupported)?;
    if aki.authority_cert_issuer.is_some() || aki.authority_cert_serial_number.is_some() {
        return Err(unsupported);
    }
    let key_id = aki.key_identifier.ok_or(unsupported)?;
    if key_id.as_bytes() != root.subject_key_id
        || tbs.issuer != root.certificate.tbs_certificate.subject
        || !signature_matches(
            &root.key,
            &digest(&encoded(tbs, malformed)?),
            crl.signature.raw_bytes(),
        )
    {
        return Err(CredentialFailure::UntrustedCrl);
    }
    let number: CrlNumber = extension(extensions, false, unsupported)?;
    if number.0.as_bytes().len() > 8 {
        return Err(unsupported);
    }
    let number = number
        .0
        .as_bytes()
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte));
    let this_update = time_to_unix(&tbs.this_update);
    let next_update = time_to_unix(&tbs.next_update.ok_or(unsupported)?);
    if next_update <= this_update {
        return Err(unsupported);
    }
    if at < this_update {
        return Err(CredentialFailure::CrlNotYetValid);
    }
    if at > next_update {
        return Err(CredentialFailure::CrlExpired);
    }
    let entries = tbs.revoked_certificates.as_deref().unwrap_or_default();
    if entries.len() > 10_000 {
        return Err(CredentialFailure::LimitExceeded);
    }
    let mut serials = HashSet::with_capacity(entries.len());
    for entry in entries {
        let serial = entry.serial_number.as_bytes();
        if serial.len() > 20
            || serial.iter().all(|byte| *byte == 0)
            || !serials.insert(serial)
            || entry.crl_entry_extensions.is_some()
            || time_to_unix(&entry.revocation_date) > this_update
        {
            return Err(unsupported);
        }
    }
    Ok(ParsedCrl {
        crl,
        der,
        number,
        this_update,
        next_update,
    })
}
