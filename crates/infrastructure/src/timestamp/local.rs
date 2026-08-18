//! Local development timestamp authority driven through the openssl
//! command line tool.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use domain::crypto::timestamp::TimestampService;
use domain::crypto::Sha256Digest;
use domain::DomainError;

use crate::error::TsaError;

use super::stderr_fragment;

/// Distinguishes the scratch files of concurrent requests within one
/// process; the process id distinguishes across processes.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// [`TimestampService`] backed by `openssl ts` subprocesses and the TSA
/// working directory prepared by `pki/issue-tsa-cert.sh`.
///
/// The adapter receives every path explicitly: the OpenSSL configuration
/// holding the `tsa` section (`pki/tsa.cnf`) and the TSA working
/// directory with the signer key, certificate, chain, and serial file.
/// It resolves nothing from its own environment; the configuration reads
/// the working directory from the TSA_DIR variable, which the adapter
/// sets on the subprocesses it spawns.
///
/// Each request writes a query file and a reply file inside the working
/// directory and removes both before returning. The reply step is
/// serialized across threads and processes with an advisory lock on
/// `serial.lock` inside the working directory, because openssl advances
/// the authority's serial file with a non-atomic read-increment-write.
#[derive(Debug, Clone)]
pub struct LocalOpensslTsa {
    config_path: PathBuf,
    tsa_dir: PathBuf,
}

impl LocalOpensslTsa {
    /// Creates the adapter over the `tsa.cnf` configuration file and the
    /// TSA working directory.
    pub fn new(config_path: impl Into<PathBuf>, tsa_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
            tsa_dir: tsa_dir.into(),
        }
    }

    fn obtain(&self, digest: &Sha256Digest) -> Result<Vec<u8>, TsaError> {
        if !self.tsa_dir.is_dir() {
            return Err(TsaError::Unreachable(format!(
                "TSA working directory not found: {} (run pki/init-ca.sh and pki/issue-tsa-cert.sh)",
                self.tsa_dir.display()
            )));
        }
        let base = format!(
            "request-{}-{}",
            std::process::id(),
            REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let query_path = self.tsa_dir.join(format!("{base}.tsq"));
        let reply_path = self.tsa_dir.join(format!("{base}.tsr"));
        let result = self.query_then_reply(digest, &query_path, &reply_path);
        let _ = fs::remove_file(&query_path);
        let _ = fs::remove_file(&reply_path);
        result
    }

    fn query_then_reply(
        &self,
        digest: &Sha256Digest,
        query_path: &Path,
        reply_path: &Path,
    ) -> Result<Vec<u8>, TsaError> {
        // Build the request over the digest; -cert asks the authority to
        // include its certificate chain in the token so a verifier only
        // needs the trust anchor.
        let mut query = Command::new("openssl");
        query
            .args([
                "ts",
                "-query",
                "-digest",
                &digest.to_hex(),
                "-sha256",
                "-cert",
                "-out",
            ])
            .arg(query_path);
        run_step("openssl ts -query", &mut query)?;

        // openssl updates the authority's serial file with a non-atomic
        // read-increment-write, so unserialized concurrent replies can
        // mint duplicate serial numbers. An exclusive cross-process
        // advisory lock over the reply step prevents that; it is
        // released when the guard drops, after the reply file is read.
        let _serial_guard = SerialLock::acquire(&self.tsa_dir)?;

        let mut reply = Command::new("openssl");
        reply
            .args(["ts", "-reply", "-config"])
            .arg(&self.config_path)
            .arg("-queryfile")
            .arg(query_path)
            .arg("-out")
            .arg(reply_path)
            .env("TSA_DIR", &self.tsa_dir);
        run_step("openssl ts -reply", &mut reply)?;

        let token = fs::read(reply_path).map_err(|err| {
            TsaError::InvalidToken(format!("the reply file could not be read: {err}"))
        })?;
        if token.is_empty() {
            return Err(TsaError::InvalidToken(
                "the reply file is empty".to_string(),
            ));
        }
        Ok(token)
    }
}

impl TimestampService for LocalOpensslTsa {
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        Ok(self.obtain(digest)?)
    }
}

/// Exclusive cross-process advisory lock on `serial.lock` inside the
/// TSA working directory, held while `openssl ts -reply` updates the
/// authority's serial file. Dropping the guard closes the file, which
/// releases the lock.
struct SerialLock {
    _file: fs::File,
}

impl SerialLock {
    fn acquire(tsa_dir: &Path) -> Result<Self, TsaError> {
        let path = tsa_dir.join("serial.lock");
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|err| {
                TsaError::Unreachable(format!(
                    "cannot open the serial lock file {}: {err}",
                    path.display()
                ))
            })?;
        fs2::FileExt::lock_exclusive(&file).map_err(|err| {
            TsaError::Unreachable(format!(
                "cannot lock the serial lock file {}: {err}",
                path.display()
            ))
        })?;
        Ok(Self { _file: file })
    }
}

/// Runs one openssl step, folding a spawn failure into
/// [`TsaError::Unreachable`] and a non-zero exit into
/// [`TsaError::Rejected`] with the step name and a stderr fragment.
fn run_step(step: &str, command: &mut Command) -> Result<(), TsaError> {
    let output = command
        .output()
        .map_err(|err| TsaError::Unreachable(format!("cannot run {step}: {err}")))?;
    if !output.status.success() {
        return Err(TsaError::Rejected(format!(
            "{step} failed ({}): {}",
            output.status,
            stderr_fragment(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_failures_map_to_unreachable() {
        let err = run_step(
            "no-such-tool",
            Command::new("openssl-binary-that-does-not-exist").arg("x"),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            TsaError::Unreachable(ref message) if message.contains("no-such-tool")
        ));
    }

    #[test]
    fn nonzero_exits_map_to_rejected_with_the_step_name() {
        let err = run_step("openssl ts -query", &mut Command::new("false")).unwrap_err();
        assert!(matches!(
            err,
            TsaError::Rejected(ref message) if message.contains("openssl ts -query")
        ));
    }

    #[test]
    fn tsa_errors_surface_as_domain_authority_failures() {
        let err: DomainError = TsaError::Rejected("policy refused".to_string()).into();
        assert_eq!(
            err,
            DomainError::TimestampAuthorityFailure(
                "timestamp authority rejected the request: policy refused".to_string()
            )
        );
    }

    #[test]
    fn a_serial_lock_whose_directory_is_absent_is_unreachable() {
        // The lock file cannot be created when its parent directory is
        // missing, and that surfaces as an unreachable authority naming
        // the serial lock file.
        let result = SerialLock::acquire(Path::new("/nonexistent-tsa-parent/inside"));
        assert!(
            matches!(result, Err(TsaError::Unreachable(ref message)) if message.contains("serial lock")),
            "a missing lock directory should surface as unreachable"
        );
    }
}
