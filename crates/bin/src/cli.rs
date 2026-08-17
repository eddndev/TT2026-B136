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
    /// Run the local HTTP document application.
    Serve(ServeArgs),
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
        /// Directory holding the certificate-authority scripts
        /// (init-ca.sh, issue-cert.sh, revoke.sh, gen-crl.sh).
        #[arg(long, default_value = "pki")]
        scripts_dir: PathBuf,
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
            Command::Serve(_) => "serve",
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

/// Runtime paths and bind address for the local HTTP application.
#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    /// TCP address listened on by the HTTP server.
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    /// Directory for encrypted documents and the audit log.
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
        /// Serial number of the certificate to revoke, in hex as printed
        /// by `pki issue` and `pki show`.
        #[arg(long)]
        serial: String,
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
    /// Use the local OpenSSL authority instead of the remote provider.
    ///
    /// The authority working directory is read from the TSA_DIR
    /// environment variable and defaults to "pki-tsa" next to the CA
    /// working directory (PKI_CA_DIR, itself defaulting to "pki-ca").
    /// Prepare it with pki/init-ca.sh and pki/issue-tsa-cert.sh.
    #[arg(long)]
    pub mock: bool,
    /// Write the timestamp token to this path instead of `<input>.tsr`.
    #[arg(long)]
    pub out: Option<PathBuf>,
    /// Directory holding the timestamp authority configuration
    /// (tsa.cnf), used with --mock.
    #[arg(long, default_value = "pki")]
    pub pki_dir: PathBuf,
}

/// Arguments for `verify`.
#[derive(Debug, clap::Args)]
pub struct VerifyArgs {
    /// Path to the original document.
    pub file: PathBuf,
    /// Path to the detached signature.
    #[arg(long)]
    pub sig: PathBuf,
    /// Path to the signer certificate.
    #[arg(long)]
    pub cert: PathBuf,
    /// Path to the certificate of the issuing authority.
    #[arg(long)]
    pub ca: PathBuf,
    /// Path to the timestamp token. Without it the token-dependent
    /// components of the report are skipped and say so.
    #[arg(long)]
    pub tsr: Option<PathBuf>,
    /// Path to the certificate revocation list. Without it revocation
    /// is not checked and the report says so.
    #[arg(long)]
    pub crl: Option<PathBuf>,
    /// Trust anchor for the timestamp token check; defaults to the
    /// --ca certificate, which anchors the internal authorities.
    #[arg(long)]
    pub tsa_cert: Option<PathBuf>,
    /// Evaluate the certificate status at this RFC 3339 instant
    /// instead of the current time.
    #[arg(long)]
    pub at: Option<String>,
}

/// Arguments for `package export`.
#[derive(Debug, clap::Args)]
pub struct PackageExportArgs {
    /// Path to the original document.
    pub file: PathBuf,
    /// Path to the detached signature.
    #[arg(long)]
    pub sig: PathBuf,
    /// Path to the timestamp token.
    #[arg(long)]
    pub tsr: PathBuf,
    /// Path to the signer certificate.
    #[arg(long)]
    pub cert: PathBuf,
    /// Path to the certificate of the issuing authority.
    #[arg(long)]
    pub ca: PathBuf,
    /// Path to the certificate revocation list to include, so the
    /// package supports an independent revocation check.
    #[arg(long)]
    pub crl: PathBuf,
    /// Path to the timestamp authority certificate chain to include,
    /// when available.
    #[arg(long)]
    pub tsa_chain: Option<PathBuf>,
    /// Output path for the package archive.
    #[arg(long)]
    pub out: PathBuf,
}

/// Evidence-package subcommands.
#[derive(Debug, Subcommand)]
pub enum PackageAction {
    /// Export an independent verification package.
    Export(PackageExportArgs),
}

/// Authentication subcommands.
#[derive(Debug, Subcommand)]
pub enum AuthAction {
    /// Measure the password-hashing cost on this hardware.
    Calibrate,
    /// Hash a password read from standard input.
    HashPassword,
    /// Verify a password read from standard input against a stored hash.
    VerifyPassword {
        /// Stored hash to check against, in PHC string format.
        #[arg(long)]
        hash: String,
    },
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
        /// File that receives the base32 secret for later verification.
        #[arg(long)]
        secret_out: PathBuf,
    },
    /// Verify a one-time-password code.
    Verify {
        /// The code to verify.
        #[arg(long)]
        code: String,
        /// File holding the base32 secret written at enrollment.
        #[arg(long)]
        secret_file: PathBuf,
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
