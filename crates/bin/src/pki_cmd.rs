//! Handlers for the `pki` command group.
//!
//! These functions wire the OpenSSL-script authority adapter and the
//! X.509 chain validator into the pki use cases and format their output.
//! The CA working directory is resolved here, in the composition root:
//! the PKI_CA_DIR environment variable when set, otherwise `pki-ca` under
//! the current working directory, matching the default the scripts under
//! `pki/` document. The script directory comes from the command line.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use application::pki::{
    GenerateCrl, InitializeCa, InspectCertificate, IssueCertificate, IssueDeclarationCertificate,
    RevokeCertificate,
};
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::{OpensslCaAdapter, X509ChainValidator};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::cli::{PkiAction, PkiIssuePurpose};

/// Dispatches a `pki` subcommand to its handler.
pub fn run(scripts_dir: &Path, action: PkiAction, json: bool) -> anyhow::Result<()> {
    match action {
        PkiAction::InitCa => init_ca(scripts_dir, json),
        PkiAction::Issue { cn, purpose } => issue(scripts_dir, &cn, purpose, json),
        PkiAction::Revoke { serial } => revoke(scripts_dir, &serial, json),
        PkiAction::GenCrl => gen_crl(scripts_dir, json),
        PkiAction::Show { cert } => show(&cert, json),
    }
}

/// CA working directory: PKI_CA_DIR when set and non-empty, otherwise
/// `pki-ca` under the current directory.
fn ca_dir() -> anyhow::Result<PathBuf> {
    if let Some(dir) = env::var_os("PKI_CA_DIR") {
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }
    Ok(env::current_dir()
        .context("cannot resolve the current directory")?
        .join("pki-ca"))
}

fn authority(scripts_dir: &Path) -> anyhow::Result<(OpensslCaAdapter, PathBuf)> {
    let ca = ca_dir()?;
    Ok((OpensslCaAdapter::new(scripts_dir, &ca), ca))
}

fn init_ca(scripts_dir: &Path, json: bool) -> anyhow::Result<()> {
    let (adapter, ca) = authority(scripts_dir)?;
    InitializeCa::new(adapter)
        .execute()
        .context("initialization of the certificate authority failed")?;
    let root_path = ca.join("ca.crt.pem");
    if json {
        let body = serde_json::json!({
            "ca_dir": ca.display().to_string(),
            "root_certificate": root_path.display().to_string(),
        });
        println!("{body}");
    } else {
        println!("certificate authority ready at {}", ca.display());
        println!("root certificate: {}", root_path.display());
    }
    Ok(())
}

fn issue(
    scripts_dir: &Path,
    common_name: &str,
    purpose: PkiIssuePurpose,
    json: bool,
) -> anyhow::Result<()> {
    let (adapter, _ca) = authority(scripts_dir)?;
    let at = now_unix()?;
    let issued = match purpose {
        PkiIssuePurpose::Partner => {
            IssueCertificate::new(adapter, X509ChainValidator::new()).execute(common_name, at)
        }
        PkiIssuePurpose::ParticipantDeclaration => IssueDeclarationCertificate::new(
            adapter.for_internal_declarations(),
            X509ChainValidator::new(),
            InternalRsaDeclarationVerifier::new(),
        )
        .execute(common_name, at),
    }
    .context("certificate issuance failed")?;
    if json {
        let body = serde_json::json!({
            "serial": issued.serial_hex,
            "certificate": issued.certificate_path.display().to_string(),
            "private_key": issued.private_key_path.display().to_string(),
        });
        println!("{body}");
    } else {
        println!("issued certificate serial {}", issued.serial_hex);
        println!("  certificate: {}", issued.certificate_path.display());
        println!(
            "  private key: {} (keep secret, never commit)",
            issued.private_key_path.display()
        );
    }
    Ok(())
}

fn revoke(scripts_dir: &Path, serial: &str, json: bool) -> anyhow::Result<()> {
    let (adapter, _ca) = authority(scripts_dir)?;
    RevokeCertificate::new(adapter)
        .execute(serial)
        .with_context(|| format!("revocation of serial {serial} failed"))?;
    if json {
        let body = serde_json::json!({ "revoked_serial": serial });
        println!("{body}");
    } else {
        println!("revoked certificate serial {serial}");
        println!("run 'pki gen-crl' to publish an updated revocation list");
    }
    Ok(())
}

fn gen_crl(scripts_dir: &Path, json: bool) -> anyhow::Result<()> {
    let (adapter, ca) = authority(scripts_dir)?;
    GenerateCrl::new(adapter)
        .execute()
        .context("revocation list generation failed")?;
    let crl_path = ca.join("crl/crl.pem");
    if json {
        let body = serde_json::json!({ "crl": crl_path.display().to_string() });
        println!("{body}");
    } else {
        println!("wrote {}", crl_path.display());
    }
    Ok(())
}

fn show(cert: &Path, json: bool) -> anyhow::Result<()> {
    let bytes = fs::read(cert).with_context(|| format!("cannot read {}", cert.display()))?;
    let summary = InspectCertificate::new(X509ChainValidator::new())
        .execute(&bytes)
        .with_context(|| format!("cannot parse {}", cert.display()))?;
    let not_before = format_time(summary.not_before_unix)?;
    let not_after = format_time(summary.not_after_unix)?;
    if json {
        let body = serde_json::json!({
            "subject": summary.subject,
            "issuer": summary.issuer,
            "serial": summary.serial_hex,
            "not_before": not_before,
            "not_after": not_after,
        });
        println!("{body}");
    } else {
        println!("subject:    {}", summary.subject);
        println!("issuer:     {}", summary.issuer);
        println!("serial:     {}", summary.serial_hex);
        println!("not before: {not_before}");
        println!("not after:  {not_after}");
    }
    Ok(())
}

fn now_unix() -> anyhow::Result<i64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?;
    Ok(now.as_secs() as i64)
}

fn format_time(unix_seconds: i64) -> anyhow::Result<String> {
    let stamp = OffsetDateTime::from_unix_timestamp(unix_seconds)
        .context("timestamp outside the representable range")?;
    stamp
        .format(&Rfc3339)
        .context("cannot format the timestamp")
}
