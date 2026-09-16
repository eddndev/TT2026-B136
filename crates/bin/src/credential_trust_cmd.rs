//! Explicit administrative publication of public credential trust material.

use anyhow::{bail, Context};
use application::credential_trust::{
    CredentialTrustExpectation, CredentialTrustPublisher, CredentialTrustRevision,
    CredentialTrustService,
};
use clap::Subcommand;
use infrastructure::{
    certificates::InternalRsaDeclarationVerifier, PostgresCredentialTrustStore, SystemClock,
};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Subcommand)]
pub enum CredentialTrustAction {
    /// Publish a root and CRL using DATABASE_URL; zero expects no prior trust.
    Publish {
        #[arg(long)]
        root_cert: PathBuf,
        #[arg(long)]
        crl: PathBuf,
        #[arg(long)]
        expected_revision: u32,
    },
}
pub fn run(action: CredentialTrustAction, json: bool) -> anyhow::Result<()> {
    let CredentialTrustAction::Publish {
        root_cert,
        crl,
        expected_revision,
    } = action;
    let url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set for credential trust publication")?;
    let root = read(&root_cert, 16 * 1024, "credential root exceeds 16 KiB")?;
    let crl = read(&crl, 1024 * 1024, "credential CRL exceeds 1 MiB")?;
    let expected = match expected_revision {
        0 => CredentialTrustExpectation::Absent,
        value => CredentialTrustExpectation::Revision(CredentialTrustRevision::new(value)?),
    };
    let clock = Arc::new(SystemClock::new());
    let store = Arc::new(
        PostgresCredentialTrustStore::open(&url, clock.clone())
            .context("cannot open administrative credential trust store")?,
    );
    let service = CredentialTrustService::new(
        store,
        Arc::new(InternalRsaDeclarationVerifier::new()),
        clock,
    );
    let row = service
        .publish(&root, &crl, expected)
        .context("credential trust publication failed")?;
    if json {
        println!(
            "{}",
            serde_json::json!({"deployment_id":row.deployment_id.to_string(),"revision":row.revision.get(),
            "root_fingerprint":row.inspection.root_fingerprint.to_hex(),"crl_digest":row.inspection.crl_digest.to_hex(),
            "crl_number":row.inspection.crl_number.to_string(),"crl_this_update_unix":row.inspection.crl_this_update,
            "crl_next_update_unix":row.inspection.crl_next_update,"published_at":row.published_at.format(&Rfc3339)?,
            "published_by":row.published_by})
        );
    } else {
        println!(
            "published internal credential trust revision {} for deployment {}",
            row.revision.get(),
            row.deployment_id
        );
        println!("root SHA-256: {}", row.inspection.root_fingerprint.to_hex());
        println!("CRL SHA-256: {}", row.inspection.crl_digest.to_hex());
    }
    Ok(())
}
fn read(path: &Path, maximum: usize, message: &str) -> anyhow::Result<Vec<u8>> {
    let file = File::open(path).context("cannot open public credential material")?;
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .context("cannot read public credential material")?;
    if bytes.len() > maximum {
        bail!("{message}");
    }
    if bytes.is_empty() {
        bail!("public credential material must not be empty");
    }
    Ok(bytes)
}
