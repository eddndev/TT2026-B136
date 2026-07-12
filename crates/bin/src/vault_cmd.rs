//! Handlers for the `vault` command group.
//!
//! These functions wire the AES-256-GCM cipher and the envelope key manager
//! into the vault use cases and format their output. Key material is read
//! from the environment here, in the composition root, so the adapters stay
//! free of environment access: KEK_BASE64 holds the current key encryption
//! key and NEW_KEK_BASE64 holds the replacement during a rotation. Both are
//! 32 random bytes encoded in standard base64.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use application::vault::{DecryptDocument, EncryptDocument, RotateKek};
use base64::Engine;
use domain::crypto::document::{DocumentId, DocumentVersion};
use domain::crypto::keys::KEY_ENCRYPTION_KEY_LEN;
use infrastructure::{EnvelopeKeyManager, RingAesGcmCipher};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::cli::VaultAction;

/// Environment variable holding the current key encryption key.
const KEK_VAR: &str = "KEK_BASE64";

/// Environment variable holding the replacement key during a rotation.
const NEW_KEK_VAR: &str = "NEW_KEK_BASE64";

/// Dispatches a `vault` subcommand to its handler. With `json` set, each
/// handler prints one JSON object instead of its human-readable line.
pub fn run(action: VaultAction, json: bool) -> anyhow::Result<()> {
    match action {
        VaultAction::Encrypt {
            file,
            doc_id,
            version,
        } => encrypt(&file, &doc_id, version, json),
        VaultAction::Decrypt {
            package,
            doc_id,
            version,
            out,
        } => decrypt(&package, &doc_id, version, out.as_deref(), json),
        VaultAction::RotateKek { file } => rotate_kek(&file, json),
    }
}

// JSON shape: {"package_path": <string>}.
fn encrypt(file: &Path, doc_id: &str, version: u32, json: bool) -> anyhow::Result<()> {
    let id = parse_doc_id(doc_id)?;
    let version = DocumentVersion::new(version)?;
    let kek = load_kek(KEK_VAR)?;
    let plaintext =
        fs::read(file).with_context(|| format!("cannot read {} for encryption", file.display()))?;

    let use_case = EncryptDocument::new(RingAesGcmCipher::new(), EnvelopeKeyManager::new());
    let vault_bytes = use_case
        .execute(&kek, &plaintext, id, version)
        .context("encryption failed")?;

    let out_path = path_with_enc_suffix(file);
    fs::write(&out_path, &vault_bytes)
        .with_context(|| format!("cannot write {}", out_path.display()))?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "package_path": out_path.display().to_string() })
        );
    } else {
        println!("wrote {}", out_path.display());
    }
    Ok(())
}

// JSON shape with --out: {"out": <string>}. Without --out the plaintext
// itself goes to standard output, so no JSON wraps it.
fn decrypt(
    package: &Path,
    doc_id: &str,
    version: u32,
    out: Option<&Path>,
    json: bool,
) -> anyhow::Result<()> {
    let id = parse_doc_id(doc_id)?;
    let version = DocumentVersion::new(version)?;
    let kek = load_kek(KEK_VAR)?;
    let vault_bytes =
        fs::read(package).with_context(|| format!("cannot read {}", package.display()))?;

    let use_case = DecryptDocument::new(RingAesGcmCipher::new(), EnvelopeKeyManager::new());
    let plaintext = use_case
        .execute(&kek, &vault_bytes, id, version)
        .with_context(|| format!("decryption of {} failed", package.display()))?;

    match out {
        Some(path) => {
            fs::write(path, &plaintext)
                .with_context(|| format!("cannot write {}", path.display()))?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "out": path.display().to_string() })
                );
            } else {
                println!("wrote {}", path.display());
            }
        }
        None => std::io::stdout()
            .write_all(&plaintext)
            .context("cannot write the plaintext to standard output")?,
    }
    Ok(())
}

// JSON shape: {"rewritten": <string>}.
fn rotate_kek(file: &Path, json: bool) -> anyhow::Result<()> {
    let old_kek = load_kek(KEK_VAR)?;
    let new_kek = load_kek(NEW_KEK_VAR)?;
    let vault_bytes = fs::read(file).with_context(|| format!("cannot read {}", file.display()))?;

    let use_case = RotateKek::new(EnvelopeKeyManager::new());
    let rotated = use_case
        .execute(&old_kek, &new_kek, &vault_bytes)
        .with_context(|| format!("key rotation of {} failed", file.display()))?;

    // Write the rotated bytes next to the original and rename over it, so a
    // failure while writing leaves the original package intact.
    let temp_path = path_with_suffix(file, ".tmp");
    fs::write(&temp_path, &rotated)
        .with_context(|| format!("cannot write {}", temp_path.display()))?;
    fs::rename(&temp_path, file).with_context(|| {
        format!(
            "cannot replace {} with {}",
            file.display(),
            temp_path.display()
        )
    })?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "rewritten": file.display().to_string() })
        );
    } else {
        println!("rewrapped the data key in {}", file.display());
    }
    Ok(())
}

/// Reads a 32-byte key from a base64-encoded environment variable into a
/// zeroizing buffer.
fn load_kek(var: &str) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let encoded =
        env::var(var).with_context(|| format!("environment variable {var} is not set"))?;
    let decoded = Zeroizing::new(
        base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .with_context(|| format!("{var} is not valid base64"))?,
    );
    anyhow::ensure!(
        decoded.len() == KEY_ENCRYPTION_KEY_LEN,
        "{var} must decode to {KEY_ENCRYPTION_KEY_LEN} bytes, got {}",
        decoded.len()
    );
    Ok(decoded)
}

fn parse_doc_id(doc_id: &str) -> anyhow::Result<DocumentId> {
    let uuid = Uuid::parse_str(doc_id).context("--doc-id must be a UUID")?;
    Ok(DocumentId::from_uuid(uuid))
}

/// Appends `.enc` to the full file name, keeping the original extension.
fn path_with_enc_suffix(file: &Path) -> PathBuf {
    path_with_suffix(file, ".enc")
}

fn path_with_suffix(file: &Path, suffix: &str) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
