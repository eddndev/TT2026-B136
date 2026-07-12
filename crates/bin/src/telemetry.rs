//! Logging setup and the secret-redaction policy.
//!
//! Redaction policy: secret material (key material, passwords, one-time-
//! password secrets, service tokens, nonces) is never passed to logging.
//! Settings loaded from the environment log presence, not values. The helper
//! that substitutes a marker for a secret is added next to the first code that
//! needs to log around one.

use tracing_subscriber::EnvFilter;

/// Initializes the global logging subscriber.
///
/// `RUST_LOG` overrides `directive` when set; otherwise `directive` is used,
/// falling back to `info` if it cannot be parsed. Safe to call once at start.
///
/// Logs go to standard error so that standard output stays reserved for
/// command results, which scripts consume directly or as JSON.
pub fn init(directive: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(directive))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
