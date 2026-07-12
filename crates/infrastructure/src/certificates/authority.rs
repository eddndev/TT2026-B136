//! Certificate authority adapter backed by the OpenSSL scripts under
//! `pki/`.
//!
//! Each operation runs one of the versioned shell scripts as a subprocess.
//! The CA working directory is injected through the PKI_CA_DIR environment
//! variable and every script is addressed inside the configured script
//! directory, so the caller's working directory never influences the
//! outcome. Script output is captured: stderr is folded into errors and
//! stdout is discarded because the scripts print human-oriented progress
//! text.

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use domain::crypto::certificate::{CertificateAuthority, IssuedCertificate};
use domain::DomainError;

use super::parse::{parse_certificate, serial_hex};

/// Longest stderr fragment copied into an error message, so a failing
/// script cannot flood logs.
const STDERR_LIMIT: usize = 600;

/// [`CertificateAuthority`] that drives the shell scripts of the internal
/// OpenSSL authority: `init-ca.sh`, `issue-cert.sh`, `revoke.sh`, and
/// `gen-crl.sh`.
#[derive(Debug, Clone)]
pub struct OpensslCaAdapter {
    scripts_dir: PathBuf,
    ca_dir: PathBuf,
}

impl OpensslCaAdapter {
    /// Creates the adapter over the directory holding the authority
    /// scripts and the CA working directory handed to them through the
    /// PKI_CA_DIR environment variable.
    pub fn new(scripts_dir: impl Into<PathBuf>, ca_dir: impl Into<PathBuf>) -> Self {
        Self {
            scripts_dir: scripts_dir.into(),
            ca_dir: ca_dir.into(),
        }
    }

    fn run_script(&self, name: &str, args: &[&OsStr]) -> Result<(), DomainError> {
        let script = self.scripts_dir.join(name);
        if !script.is_file() {
            return Err(authority_failure(format!(
                "script not found: {}",
                script.display()
            )));
        }
        let output = Command::new("bash")
            .arg(&script)
            .args(args)
            .env("PKI_CA_DIR", &self.ca_dir)
            .output()
            .map_err(|err| authority_failure(format!("cannot run {name}: {err}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let fragment: String = stderr.trim().chars().take(STDERR_LIMIT).collect();
            return Err(authority_failure(format!(
                "{name} failed ({}): {fragment}",
                output.status
            )));
        }
        Ok(())
    }

    fn read_ca_file(&self, relative: &str) -> Result<Vec<u8>, DomainError> {
        let path = self.ca_dir.join(relative);
        fs::read(&path)
            .map_err(|err| authority_failure(format!("cannot read {}: {err}", path.display())))
    }
}

impl CertificateAuthority for OpensslCaAdapter {
    fn init_ca(&self) -> Result<Vec<u8>, DomainError> {
        self.run_script("init-ca.sh", &[])?;
        self.read_ca_file("ca.crt.pem")
    }

    fn root_certificate(&self) -> Result<Vec<u8>, DomainError> {
        self.read_ca_file("ca.crt.pem")
    }

    fn issue(&self, common_name: &str) -> Result<IssuedCertificate, DomainError> {
        self.run_script("issue-cert.sh", &[OsStr::new(common_name)])?;
        let slug = file_slug(common_name);
        let certificate_path = self.ca_dir.join("certs").join(format!("{slug}.crt.pem"));
        let private_key_path = self.ca_dir.join("private").join(format!("{slug}.key.pem"));
        let certificate_pem = fs::read(&certificate_path).map_err(|err| {
            authority_failure(format!(
                "issue-cert.sh succeeded but the certificate is unreadable at {}: {err}",
                certificate_path.display()
            ))
        })?;
        let certificate = parse_certificate(&certificate_pem)?;
        Ok(IssuedCertificate {
            serial_hex: serial_hex(&certificate.tbs_certificate.serial_number),
            certificate_pem,
            certificate_path,
            private_key_path,
        })
    }

    fn revoke(&self, serial_hex: &str) -> Result<(), DomainError> {
        let normalized = normalize_serial(serial_hex)?;
        // openssl ca keeps a copy of every issued certificate under
        // newcerts/<SERIAL>.pem; that copy maps the serial back to a file
        // that revoke.sh accepts.
        let cert_path = self
            .ca_dir
            .join("newcerts")
            .join(format!("{normalized}.pem"));
        if !cert_path.is_file() {
            return Err(authority_failure(format!(
                "no issued certificate with serial {normalized}"
            )));
        }
        self.run_script("revoke.sh", &[cert_path.as_os_str()])
    }

    fn generate_crl(&self) -> Result<Vec<u8>, DomainError> {
        self.run_script("gen-crl.sh", &[])?;
        self.read_ca_file("crl/crl.pem")
    }
}

/// Mirrors the file-name slug that issue-cert.sh derives from the common
/// name: ASCII lowercase with spaces replaced by dashes.
fn file_slug(common_name: &str) -> String {
    common_name
        .chars()
        .map(|c| {
            if c == ' ' {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

/// Validates a serial given as hex digits and normalizes it to the
/// uppercase form used by the CA database and its file names.
fn normalize_serial(serial: &str) -> Result<String, DomainError> {
    if serial.is_empty() || !serial.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(authority_failure(format!(
            "serial must be hex digits, got {serial:?}"
        )));
    }
    Ok(serial.to_ascii_uppercase())
}

fn authority_failure(message: String) -> DomainError {
    DomainError::CertificateAuthorityFailure(message)
}

#[cfg(test)]
mod tests {
    use domain::crypto::certificate::CertificateAuthority;
    use domain::DomainError;

    use super::{file_slug, normalize_serial, OpensslCaAdapter};

    fn assert_failure_mentions(result: Result<Vec<u8>, DomainError>, needle: &str) {
        match result {
            Err(DomainError::CertificateAuthorityFailure(message)) => {
                assert!(
                    message.contains(needle),
                    "message {message:?} should mention {needle:?}"
                );
            }
            other => panic!("expected an authority failure, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_script_directory_is_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = OpensslCaAdapter::new(dir.path().join("no-scripts"), dir.path().join("ca"));
        assert_failure_mentions(adapter.init_ca(), "script not found");
    }

    #[test]
    fn reading_the_root_before_initialization_fails() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = OpensslCaAdapter::new(dir.path(), dir.path().join("ca"));
        assert_failure_mentions(adapter.root_certificate(), "cannot read");
    }

    #[test]
    fn revoking_a_non_hex_serial_is_rejected_before_any_subprocess() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = OpensslCaAdapter::new(dir.path(), dir.path().join("ca"));
        let err = adapter.revoke("not-hex").unwrap_err();
        assert!(matches!(
            err,
            DomainError::CertificateAuthorityFailure(ref message)
                if message.contains("hex digits")
        ));
    }

    #[test]
    fn revoking_an_unissued_serial_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = OpensslCaAdapter::new(dir.path(), dir.path().join("ca"));
        let err = adapter.revoke("10ff").unwrap_err();
        assert!(matches!(
            err,
            DomainError::CertificateAuthorityFailure(ref message)
                if message.contains("no issued certificate with serial 10FF")
        ));
    }

    #[test]
    fn the_slug_matches_the_script_derivation() {
        assert_eq!(file_slug("Juan Perez"), "juan-perez");
        assert_eq!(file_slug("despacho_2026@interno"), "despacho_2026@interno");
    }

    #[test]
    fn serials_normalize_to_uppercase() {
        assert_eq!(normalize_serial("10a3").unwrap(), "10A3");
    }
}
