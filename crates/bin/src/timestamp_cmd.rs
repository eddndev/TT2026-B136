//! Handler for the `timestamp` command.
//!
//! Wires the SHA-256 hasher and a timestamp authority adapter into the
//! timestamping use case: the input file is digested, a token is
//! obtained, checked to cover that digest, written next to the input as
//! `<input>.tsr` (or to `--out`), and its generation data is printed.
//! With `--mock` the authority is the local OpenSSL one under the TSA
//! working directory; otherwise the remote provider is reached with the
//! credentials from the environment.

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use application::timestamping::TimestampDocument;
use domain::crypto::timestamp::TimestampService;
use infrastructure::timestamp::{token_info, RetryPolicy};
use infrastructure::{CincelTsaAdapter, LocalOpensslTsa, RingSha256Hasher};
use zeroize::Zeroizing;

use crate::cli::TimestampArgs;

/// How long each request to the remote provider may take.
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(15);

/// How often and how many times a still-processing token is polled.
const PROVIDER_RETRY: RetryPolicy = RetryPolicy {
    max_attempts: 5,
    delay: Duration::from_secs(2),
};

/// Obtains a timestamp token for a file and writes it as `<input>.tsr`
/// (or to `--out`).
///
/// Human-readable output is the digest hex, the token path, and the
/// generation time. With `json` set, the output is one object carrying
/// the same three fields.
pub fn run(args: &TimestampArgs, json: bool) -> anyhow::Result<()> {
    if args.mock {
        let config = args.pki_dir.join("tsa.cnf");
        anyhow::ensure!(
            config.is_file(),
            "TSA configuration not found at {} (point --pki-dir at the pki directory)",
            config.display()
        );
        let tsa_dir = resolve_tsa_dir();
        anyhow::ensure!(
            tsa_dir.is_dir(),
            "TSA working directory not found at {} (set TSA_DIR or run pki/init-ca.sh and pki/issue-tsa-cert.sh)",
            tsa_dir.display()
        );
        timestamp_with(LocalOpensslTsa::new(config, tsa_dir), args, json)
    } else {
        let settings = non_empty_env("CINCEL_BASE_URL").zip(non_empty_env("CINCEL_API_KEY"));
        let Some((base_url, api_key)) = settings else {
            anyhow::bail!(
                "CINCEL_BASE_URL and CINCEL_API_KEY are not configured; set them in the \
                 environment (or .env) to use the remote provider, or pass --mock to use \
                 the local authority"
            );
        };
        let adapter = CincelTsaAdapter::new(
            base_url,
            Zeroizing::new(api_key),
            PROVIDER_TIMEOUT,
            PROVIDER_RETRY,
        )
        .context("cannot set up the provider client")?;
        timestamp_with(adapter, args, json)
    }
}

fn timestamp_with<T: TimestampService>(
    service: T,
    args: &TimestampArgs,
    json: bool,
) -> anyhow::Result<()> {
    let mut input = File::open(&args.input)
        .with_context(|| format!("cannot open file: {}", args.input.display()))?;
    let stamped = TimestampDocument::new(RingSha256Hasher::new(), service)
        .execute(&mut input)
        .context("timestamping failed")?;

    // Read the token back before writing anything, so an authority that
    // answered with an unusable token or one covering a different
    // digest fails here instead of leaving a bogus file behind.
    let info = token_info(&stamped.token).context("the authority returned an unusable token")?;
    anyhow::ensure!(
        info.digest_hex == stamped.digest.to_hex(),
        "the token covers digest {} instead of {}",
        info.digest_hex,
        stamped.digest.to_hex()
    );

    let token_path = args
        .out
        .clone()
        .unwrap_or_else(|| path_with_tsr_suffix(&args.input));
    fs::write(&token_path, &stamped.token)
        .with_context(|| format!("cannot write {}", token_path.display()))?;

    if json {
        let body = serde_json::json!({
            "digest": stamped.digest.to_hex(),
            "token_path": token_path.display().to_string(),
            "generated_at": info.generated_at,
        });
        println!("{body}");
    } else {
        println!("digest: {}", stamped.digest.to_hex());
        println!("token: {}", token_path.display());
        println!("generated at: {}", info.generated_at);
    }
    Ok(())
}

/// Resolves the TSA working directory: TSA_DIR when set, otherwise the
/// default next to the CA working directory.
fn resolve_tsa_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("TSA_DIR") {
        return PathBuf::from(dir);
    }
    let ca_dir = non_empty_env("PKI_CA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("pki-ca"));
    default_tsa_dir(&ca_dir)
}

/// The default TSA working directory sits next to the CA working
/// directory, mirroring the default of pki/issue-tsa-cert.sh.
fn default_tsa_dir(ca_dir: &Path) -> PathBuf {
    match ca_dir.parent() {
        Some(parent) => parent.join("pki-tsa"),
        None => PathBuf::from("pki-tsa"),
    }
}

/// Reads an environment variable, treating a blank value as absent so
/// the placeholder entries of `.env.example` do not count as settings.
fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

/// Appends `.tsr` to the full file name, keeping the original extension.
fn path_with_tsr_suffix(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".tsr");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{default_tsa_dir, path_with_tsr_suffix};

    #[test]
    fn the_default_tsa_directory_sits_next_to_the_ca_directory() {
        assert_eq!(
            default_tsa_dir(Path::new("/srv/pki-ca")),
            Path::new("/srv/pki-tsa")
        );
        assert_eq!(default_tsa_dir(Path::new("pki-ca")), Path::new("pki-tsa"));
    }

    #[test]
    fn the_token_suffix_keeps_the_original_extension() {
        assert_eq!(
            path_with_tsr_suffix(Path::new("acta.txt")),
            Path::new("acta.txt.tsr")
        );
    }
}
