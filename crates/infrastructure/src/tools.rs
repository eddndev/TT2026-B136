//! Interrogation of the external tools the adapters delegate to.

use std::process::Command;

use crate::error::CryptoError;

/// Returns the version line of the local openssl binary, as printed by
/// `openssl version`. Evidence packages quote it so a reader knows what
/// tool the documented verification commands were written against.
pub fn openssl_version() -> Result<String, CryptoError> {
    let output = Command::new("openssl")
        .arg("version")
        .output()
        .map_err(|err| CryptoError::Backend(format!("cannot run openssl version: {err}")))?;
    if !output.status.success() {
        return Err(CryptoError::Backend(format!(
            "openssl version failed ({})",
            output.status
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err(CryptoError::Backend(
            "openssl version printed nothing".to_string(),
        ));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::openssl_version;

    #[test]
    fn the_local_openssl_reports_a_version_line() {
        let version = openssl_version().expect("openssl is available in this environment");
        assert!(
            version.starts_with("OpenSSL"),
            "unexpected version line: {version}"
        );
    }
}
