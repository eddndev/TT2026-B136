//! OpenSSL issuance from a verified holder-generated public CSR.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;

use der::Encode;
use domain::crypto::certificate::{CertificateRequestAuthority, IssuedPublicCertificate};
use domain::DomainError;

use super::parse::{parse_certificate, serial_hex};
use super::request_material::{failure, inspect, validate_name, REQUEST_LIMIT};
use super::OpensslCaAdapter;

impl CertificateRequestAuthority for OpensslCaAdapter {
    fn issue_request(
        &self,
        name: &str,
        request_pem: &[u8],
    ) -> Result<IssuedPublicCertificate, DomainError> {
        validate_name(name)?;
        let expected_key = inspect(request_pem)?;
        let directory =
            tempfile::tempdir().map_err(|_| failure("public staging directory is unavailable"))?;
        let request_path = directory.path().join("request.pem");
        fs::write(&request_path, request_pem)
            .map_err(|_| failure("public staging file is unavailable"))?;
        self.run_script(
            "issue-request.sh",
            &[OsStr::new(name), request_path.as_os_str()],
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
        if actual_key != expected_key {
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
