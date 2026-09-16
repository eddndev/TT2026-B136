use der::oid::AssociatedOid;
use der::Decode;
use domain::crypto::{CertificateSummary, CredentialCertificate, CredentialFailure};
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPublicKey;
use x509_cert::ext::pkix::{
    AuthorityKeyIdentifier, BasicConstraints, KeyUsage, KeyUsages, SubjectKeyIdentifier,
};
use x509_cert::{Certificate, Version};

use super::material::{
    der_bytes, digest, encoded, extension, inventory, signature_matches, CERTIFICATE_LIMIT,
    RSA_ENCRYPTION, RSA_SHA256,
};
use crate::certificates::parse::{serial_hex, time_to_unix};

pub(super) struct ParsedCertificate {
    pub certificate: Certificate,
    pub der: Vec<u8>,
    pub key: RsaPublicKey,
    pub subject_key_id: Vec<u8>,
    pub authority_key_id: Option<Vec<u8>>,
    pub not_before: i64,
    pub not_after: i64,
}

impl ParsedCertificate {
    pub fn inspection(&self) -> CredentialCertificate {
        let tbs = &self.certificate.tbs_certificate;
        CredentialCertificate {
            fingerprint: digest(&self.der),
            der: self.der.clone(),
            summary: CertificateSummary {
                subject: tbs.subject.to_string(),
                issuer: tbs.issuer.to_string(),
                serial_hex: serial_hex(&tbs.serial_number),
                not_before_unix: self.not_before,
                not_after_unix: self.not_after,
            },
        }
    }

    pub fn check_time(&self, at: i64) -> Result<(), CredentialFailure> {
        if at < self.not_before {
            return Err(CredentialFailure::NotYetValid);
        }
        if at > self.not_after {
            return Err(CredentialFailure::Expired);
        }
        Ok(())
    }

    pub fn check_issuer(&self, root: &Self) -> Result<(), CredentialFailure> {
        let tbs = &self.certificate.tbs_certificate;
        if tbs.issuer != root.certificate.tbs_certificate.subject
            || self.authority_key_id.as_ref() != Some(&root.subject_key_id)
            || !signature_matches(
                &root.key,
                &digest(&encoded(tbs, CredentialFailure::MalformedCertificate)?),
                self.certificate.signature.raw_bytes(),
            )
        {
            return Err(CredentialFailure::UntrustedIssuer);
        }
        Ok(())
    }
}

pub(super) fn certificate(
    input: &[u8],
    root: bool,
) -> Result<ParsedCertificate, CredentialFailure> {
    let malformed = CredentialFailure::MalformedCertificate;
    let unsupported = CredentialFailure::UnsupportedCertificate;
    let der = der_bytes(input, CERTIFICATE_LIMIT, "CERTIFICATE", malformed)?;
    let certificate = Certificate::from_der(&der).map_err(|_| malformed)?;
    let tbs = &certificate.tbs_certificate;
    if tbs.version != Version::V3
        || tbs.issuer_unique_id.is_some()
        || tbs.subject_unique_id.is_some()
        || tbs.subject.is_empty()
        || tbs.issuer.is_empty()
        || tbs.signature != certificate.signature_algorithm
        || tbs.signature.oid != RSA_SHA256
        || !tbs
            .signature
            .parameters
            .as_ref()
            .is_some_and(|v| v.is_null())
        || certificate.signature.as_bytes().is_none()
        || certificate.signature.raw_bytes().len() != 384
        || tbs.serial_number.as_bytes().len() > 20
        || tbs.serial_number.as_bytes().iter().all(|byte| *byte == 0)
    {
        return Err(unsupported);
    }
    let spki = &tbs.subject_public_key_info;
    if spki.algorithm.oid != RSA_ENCRYPTION
        || !spki
            .algorithm
            .parameters
            .as_ref()
            .is_some_and(|v| v.is_null())
    {
        return Err(unsupported);
    }
    let key =
        RsaPublicKey::from_public_key_der(&encoded(spki, malformed)?).map_err(|_| unsupported)?;
    if key.n().bits() != 3072 || key.e().to_bytes_be() != [1, 0, 1] {
        return Err(unsupported);
    }
    let extensions = tbs.extensions.as_deref().ok_or(unsupported)?;
    let root_oids = [
        BasicConstraints::OID,
        KeyUsage::OID,
        SubjectKeyIdentifier::OID,
    ];
    let leaf_oids = [
        BasicConstraints::OID,
        KeyUsage::OID,
        SubjectKeyIdentifier::OID,
        AuthorityKeyIdentifier::OID,
    ];
    inventory(
        extensions,
        if root { &root_oids } else { &leaf_oids },
        unsupported,
    )?;
    let constraints: BasicConstraints = extension(extensions, true, unsupported)?;
    let usage: KeyUsage = extension(extensions, true, unsupported)?;
    let expected_usage = if root {
        KeyUsages::KeyCertSign | KeyUsages::CRLSign
    } else {
        KeyUsages::DigitalSignature | KeyUsages::NonRepudiation
    };
    if constraints.ca != root
        || constraints.path_len_constraint.is_some()
        || usage.0 != expected_usage
    {
        return Err(unsupported);
    }
    let ski: SubjectKeyIdentifier = extension(extensions, false, unsupported)?;
    let key_bytes = spki.subject_public_key.as_bytes().ok_or(unsupported)?;
    // OpenSSL uses this SHA-1 public-key identifier in the fixed profile.
    // Signatures and material fingerprints are independently bound by SHA-256.
    let calculated = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, key_bytes);
    if ski.0.as_bytes() != calculated.as_ref() {
        return Err(unsupported);
    }
    let authority_key_id = if root {
        None
    } else {
        let aki: AuthorityKeyIdentifier = extension(extensions, false, unsupported)?;
        if aki.authority_cert_issuer.is_some() || aki.authority_cert_serial_number.is_some() {
            return Err(unsupported);
        }
        let value = aki.key_identifier.ok_or(unsupported)?;
        if value.as_bytes().len() != 20 {
            return Err(unsupported);
        }
        Some(value.as_bytes().to_vec())
    };
    let not_before = time_to_unix(&tbs.validity.not_before);
    let not_after = time_to_unix(&tbs.validity.not_after);
    if not_after < not_before {
        return Err(unsupported);
    }
    if root
        && (tbs.issuer != tbs.subject
            || !signature_matches(
                &key,
                &digest(&encoded(tbs, malformed)?),
                certificate.signature.raw_bytes(),
            ))
    {
        return Err(CredentialFailure::UntrustedIssuer);
    }
    Ok(ParsedCertificate {
        certificate,
        der,
        key,
        subject_key_id: ski.0.as_bytes().to_vec(),
        authority_key_id,
        not_before,
        not_after,
    })
}
