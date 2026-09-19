//! Configuration accepted by the HTTP server command.

use std::path::PathBuf;

/// Runtime paths, address, and resource limits for the HTTP application.
#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    /// Absolute or relative path to the pinned native qpdf library.
    #[arg(long, env = "DOCUMENT_QPDF_LIBRARY")]
    pub qpdf_library: PathBuf,
    /// TCP address listened on by the HTTP server.
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    /// Maximum API requests admitted before body extraction.
    #[arg(long, default_value_t = web::HttpLimits::default().max_requests)]
    pub max_in_flight_requests: std::num::NonZeroUsize,
    /// Maximum concurrent blocking application operations.
    #[arg(long, default_value_t = web::HttpLimits::default().max_blocking_operations)]
    pub max_blocking_operations: std::num::NonZeroUsize,
    /// Maximum entries dispatched and jobs attempted per serial deadline cycle.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub deadline_page_limit: u32,
    /// Positive pause in milliseconds between serial deadline cycles.
    #[arg(long, default_value = "1000")]
    pub deadline_poll_ms: std::num::NonZeroU32,
    /// Maximum reconciliation or activation actions per serial alert cycle.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
    pub alert_page_limit: u32,
    /// Positive pause in milliseconds between serial alert cycles.
    #[arg(long, default_value = "1000")]
    pub alert_poll_ms: std::num::NonZeroU32,
    /// Optional alert sender; requires ALERT_LOGIN_URL and RESEND_API_KEY together.
    #[arg(long, env = "ALERT_EMAIL_FROM")]
    pub alert_email_from: Option<String>,
    /// Public login URL used in generic alert emails.
    #[arg(long, env = "ALERT_LOGIN_URL")]
    pub alert_login_url: Option<String>,
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
