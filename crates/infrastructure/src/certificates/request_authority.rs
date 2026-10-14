//! OpenSSL issuance from a verified holder-generated public CSR.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;

use der::Encode;
use domain::crypto::certificate::{
    CertificateAuthority, CertificateRequestAuthority, CertificateRequestProfile,
    IssuedPublicCertificate,
};
use domain::crypto::DocumentHasher;
use domain::DomainError;

use super::parse::{parse_certificate, serial_hex};
use super::request_material::{failure, validate_name, REQUEST_LIMIT};
use super::{inspect_ca_request, OpensslCaAdapter};
use crate::RingSha256Hasher;

impl CertificateRequestAuthority for OpensslCaAdapter {
    fn issue_request(
        &self,
        name: &str,
        request_pem: &[u8],
        profile: CertificateRequestProfile,
    ) -> Result<IssuedPublicCertificate, DomainError> {
        validate_name(name)?;
        let inspected = inspect_ca_request(request_pem, &self.root_certificate()?)?;
        if !inspected.issuer_policy_matches {
            return Err(failure("subject does not match the issuer policy"));
        }
        let directory =
            tempfile::tempdir().map_err(|_| failure("public staging directory is unavailable"))?;
        let request_path = directory.path().join("request.pem");
        fs::write(&request_path, request_pem)
            .map_err(|_| failure("public staging file is unavailable"))?;
        let purpose = match profile {
            CertificateRequestProfile::DocumentSigning => "document-signing",
            CertificateRequestProfile::Partner => "partner",
        };
        self.run_script(
            "issue-request.sh",
            &[
                OsStr::new(name),
                request_path.as_os_str(),
                OsStr::new(purpose),
            ],
        )?;
        let certificate_path = self.ca_dir.join("certs").join(format!("{name}.crt.pem"));
        let mut certificate_pem = Vec::new();
        File::open(&certificate_path)
            .map_err(|_| failure("issued certificate is unavailable"))?
            .take(REQUEST_LIMIT as u64 + 1)
            .read_to_end(&mut certificate_pem)
            .map_err(|_| failure("issued certificate is unreadable"))?;
        if certificate_pem.len() > REQUEST_LIMIT {
            return Err(failure(
                "issued certificate exceeds the public material limit",
            ));
        }
        let certificate = parse_certificate(&certificate_pem)?;
        let actual_key = certificate
            .tbs_certificate
            .subject_public_key_info
            .to_der()
            .map_err(|_| failure("issued public key is malformed"))?;
        if RingSha256Hasher.hash_bytes(&actual_key) != inspected.spki_sha256 {
            return Err(failure(
                "issued certificate public key does not match the signed request",
            ));
        }
        Ok(IssuedPublicCertificate {
            serial_hex: serial_hex(&certificate.tbs_certificate.serial_number),
            certificate_pem,
            certificate_path,
        })
    }
}
