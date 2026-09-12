//! Configuration accepted by the HTTP server command.

use std::path::PathBuf;

/// Runtime paths, address, and resource limits for the HTTP application.
#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    /// TCP address listened on by the HTTP server.
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    /// Maximum API requests admitted before body extraction.
    #[arg(long, default_value_t = web::HttpLimits::default().max_requests)]
    pub max_in_flight_requests: std::num::NonZeroUsize,
    /// Maximum concurrent blocking application operations.
    #[arg(long, default_value_t = web::HttpLimits::default().max_blocking_operations)]
    pub max_blocking_operations: std::num::NonZeroUsize,
    /// Legacy storage directory checked for a completed import before startup.
    #[arg(long, default_value = "runtime-data")]
    pub data_dir: PathBuf,
    /// PEM certificate corresponding to the signing private key.
    #[arg(long)]
    pub signer_cert: PathBuf,
    /// PEM private key used to sign document digests.
    #[arg(long)]
    pub signer_key: PathBuf,
    /// PEM certificate of the internal issuing authority.
    #[arg(long, default_value = "pki-ca/ca.crt.pem")]
    pub ca_cert: PathBuf,
    /// Current PEM certificate revocation list.
    #[arg(long, default_value = "pki-ca/crl/crl.pem")]
    pub crl: PathBuf,
    /// OpenSSL configuration containing the local TSA section.
    #[arg(long, default_value = "pki/tsa.cnf")]
    pub tsa_config: PathBuf,
    /// Working directory of the local timestamp authority.
    #[arg(long, default_value = "pki-tsa")]
    pub tsa_dir: PathBuf,
}
