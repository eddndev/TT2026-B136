//! Command tree for the case-management prototype.
//!
//! Every leaf command maps to one use case. Parsing and help text live here;
//! the handlers are wired in the composition root as the use cases land.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Top-level command line.
#[derive(Debug, Parser)]
#[command(
    name = "despacho-cli",
    version,
    about = "Command line for the case-management prototype"
)]
pub struct Cli {
    /// Emit machine-readable JSON instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Top-level command groups.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Content-integrity operations.
    Crypto {
        #[command(subcommand)]
        action: CryptoAction,
    },
    /// Encrypt, decrypt, and rotate keys for stored documents.
    Vault {
        #[command(subcommand)]
        action: VaultAction,
    },
    /// Manage the internal certificate authority.
    Pki {
        #[command(subcommand)]
        action: PkiAction,
    },
    /// Sign a document with a partner certificate and private key.
    Sign(SignArgs),
    /// Request a trusted timestamp for a signature or a hash.
    Timestamp(TimestampArgs),
    /// Verify a document, its signature, certificate, and timestamp.
    Verify(VerifyArgs),
    /// Build an independent verification package.
    Package {
        #[command(subcommand)]
        action: PackageAction,
    },
    /// Credential hashing and second-factor enrollment.
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Append-only audit-trail operations.
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
}

impl Command {
    /// Stable short name of the command group, used for logging.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Crypto { .. } => "crypto",
            Command::Vault { .. } => "vault",
            Command::Pki { .. } => "pki",
            Command::Sign(_) => "sign",
            Command::Timestamp(_) => "timestamp",
            Command::Verify(_) => "verify",
            Command::Package { .. } => "package",
            Command::Auth { .. } => "auth",
            Command::Audit { .. } => "audit",
        }
    }
}

/// Integrity subcommands.
#[derive(Debug, Subcommand)]
pub enum CryptoAction {
    /// Print the SHA-256 hash of a file.
    Hash {
        /// Path to the file to hash.
        file: PathBuf,
    },
}

/// Confidentiality subcommands.
#[derive(Debug, Subcommand)]
pub enum VaultAction {
    /// Encrypt a file, binding it to a document identity and version.
    Encrypt {
        /// Path to the plaintext file.
        file: PathBuf,
        /// Document identity (UUID) the ciphertext is bound to.
        #[arg(long)]
        doc_id: String,
        /// Document version the ciphertext is bound to.
        #[arg(long, default_value_t = 1)]
        version: u32,
    },
    /// Decrypt a package produced by `encrypt`.
    Decrypt {
        /// Path to the encrypted package.
        package: PathBuf,
        /// Document identity (UUID) expected inside the package.
        #[arg(long)]
        doc_id: String,
        /// Document version expected inside the package.
        #[arg(long, default_value_t = 1)]
        version: u32,
        /// Write the plaintext to this path instead of standard output.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Rotate the key-encrypting key of an encrypted package in place.
    ///
    /// Reads the current key from the KEK_BASE64 environment variable and
    /// its replacement from NEW_KEK_BASE64, then rewraps the stored data
    /// key; the encrypted document itself is not touched.
    RotateKek {
        /// Path to the encrypted package to rewrap.
        #[arg(long)]
        file: PathBuf,
    },
}

/// Certificate-authority subcommands.
#[derive(Debug, Subcommand)]
pub enum PkiAction {
    /// Create the internal root certificate authority.
    InitCa,
    /// Issue a partner certificate.
    Issue {
        /// Common name for the certificate subject.
        #[arg(long)]
        cn: String,
    },
    /// Revoke a certificate by serial number.
    Revoke {
        /// Serial number of the certificate to revoke.
        #[arg(long)]
        serial: u64,
    },
    /// Generate the certificate revocation list.
    GenCrl,
    /// Print the contents of a certificate.
    Show {
        /// Path to the certificate file.
        cert: PathBuf,
    },
}

/// Arguments for `sign`.
#[derive(Debug, clap::Args)]
pub struct SignArgs {
    /// Path to the document to sign.
    pub file: PathBuf,
    /// Path to the signer certificate.
    #[arg(long)]
    pub cert: PathBuf,
    /// Path to the signer private key.
    #[arg(long)]
    pub key: PathBuf,
}

/// Arguments for `timestamp`.
#[derive(Debug, clap::Args)]
pub struct TimestampArgs {
    /// Path to the signature or hash to timestamp.
    pub input: PathBuf,
    /// Use a local mock authority instead of the real one.
    #[arg(long)]
    pub mock: bool,
}

/// Arguments for `verify`.
#[derive(Debug, clap::Args)]
pub struct VerifyArgs {
    /// Path to the original document.
    pub file: PathBuf,
    /// Path to the signature.
    #[arg(long)]
    pub sig: PathBuf,
    /// Path to the timestamp token.
    #[arg(long)]
    pub tsr: PathBuf,
    /// Path to the signer certificate.
    #[arg(long)]
    pub cert: PathBuf,
}

/// Evidence-package subcommands.
#[derive(Debug, Subcommand)]
pub enum PackageAction {
    /// Export an independent verification package.
    Export {
        /// Output path for the package archive.
        #[arg(long)]
        out: PathBuf,
    },
}

/// Authentication subcommands.
#[derive(Debug, Subcommand)]
pub enum AuthAction {
    /// Measure the password-hashing cost on this hardware.
    Calibrate,
    /// Hash a password read from standard input.
    HashPassword,
    /// Verify a password against a stored hash.
    VerifyPassword,
    /// One-time-password subcommands.
    Totp {
        #[command(subcommand)]
        action: TotpAction,
    },
}

/// One-time-password subcommands.
#[derive(Debug, Subcommand)]
pub enum TotpAction {
    /// Enroll a user and print the provisioning URI.
    Enroll {
        /// User identifier (email) to enroll.
        #[arg(long)]
        user: String,
    },
    /// Verify a one-time-password code.
    Verify {
        /// The code to verify.
        #[arg(long)]
        code: String,
    },
}

/// Audit-trail subcommands.
#[derive(Debug, Subcommand)]
pub enum AuditAction {
    /// Append an event to the audit trail.
    Append {
        /// Action performed.
        #[arg(long)]
        action: String,
        /// Resource the action applied to.
        #[arg(long)]
        resource: String,
    },
    /// Verify the full audit chain.
    VerifyChain,
    /// Print the audit trail.
    Show,
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn command_tree_is_valid() {
        Cli::command().debug_assert();
    }
}
