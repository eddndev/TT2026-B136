//! Adapters for trusted timestamping (RFC 3161).
//!
//! The local adapter drives the openssl command line tool against the
//! TSA working directory prepared by the scripts under `pki/`, for
//! development and demonstration. The verifier parses tokens natively
//! and delegates the cryptographic check to openssl
//! (docs/adr/0005-rfc3161-verification-strategy.md). Tokens travel as
//! opaque DER bytes, as the domain ports demand.

mod cincel;
mod local;
mod verify;

pub use cincel::{CincelTsaAdapter, RetryPolicy};
pub use local::LocalOpensslTsa;
pub use verify::{token_info, Rfc3161Verifier, TimestampTokenInfo};

/// Longest stderr fragment copied into an error message or a
/// verification outcome, so a failing subprocess cannot flood logs.
const STDERR_LIMIT: usize = 600;

/// Trims and truncates a subprocess's stderr for inclusion in messages.
fn stderr_fragment(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(STDERR_LIMIT)
        .collect()
}
