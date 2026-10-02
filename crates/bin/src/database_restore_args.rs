//! Explicit target and predecessor for an administrative restoration receipt.

use clap::{ArgGroup, Args};
use domain::crypto::Sha256Digest;
use uuid::Uuid;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("audit_predecessor")
    .required(true)
    .multiple(false)
    .args(["expected_empty_audit", "expected_audit_sequence"])))]
pub struct InvalidateRestoredPasswordResetsArgs {
    /// Canonical non-nil UUID retained unchanged across uncertain retries.
    #[arg(long, value_parser = operation_id)]
    pub operation_id: Uuid,
    /// Exact restored database name.
    #[arg(long, value_parser = nonempty)]
    pub expected_database: String,
    /// Exact application schema, independent of search-path prefixes.
    #[arg(long, value_parser = nonempty)]
    pub expected_schema: String,
    /// Assert that the original audit chain is empty.
    #[arg(long, conflicts_with = "expected_audit_head")]
    pub expected_empty_audit: bool,
    /// Zero-based sequence of the original audit head, at most i64::MAX.
    #[arg(long, value_parser = sequence, requires = "expected_audit_head")]
    pub expected_audit_sequence: Option<u64>,
    /// Canonical lowercase SHA-256 of the original audit head.
    #[arg(long, value_parser = head, requires = "expected_audit_sequence")]
    pub expected_audit_head: Option<Sha256Digest>,
}

fn operation_id(value: &str) -> Result<Uuid, &'static str> {
    let parsed = Uuid::parse_str(value).map_err(|_| "expected a canonical non-nil UUID")?;
    if parsed.is_nil() || parsed.to_string() != value {
        return Err("expected a canonical non-nil UUID");
    }
    Ok(parsed)
}

fn nonempty(value: &str) -> Result<String, &'static str> {
    if value.is_empty() {
        return Err("expected an explicit nonempty target name");
    }
    Ok(value.to_owned())
}

fn sequence(value: &str) -> Result<u64, &'static str> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| "expected a sequence from 0 to i64::MAX")?;
    if parsed > i64::MAX as u64 {
        return Err("expected a sequence from 0 to i64::MAX");
    }
    Ok(parsed)
}

fn head(value: &str) -> Result<Sha256Digest, &'static str> {
    let parsed = Sha256Digest::from_hex(value)
        .map_err(|_| "expected 64 lowercase hexadecimal digest characters")?;
    if parsed.to_hex() != value {
        return Err("expected 64 lowercase hexadecimal digest characters");
    }
    Ok(parsed)
}
