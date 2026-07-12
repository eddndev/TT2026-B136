//! Handlers for the `audit` command group.
//!
//! These functions wire the file audit log, the SHA-256 hasher, and the
//! system clock into the audit use cases and format their output. The
//! composition root reads the environment: AUDIT_LOG_PATH names the log
//! file (default audit-log.jsonl in the working directory) and USER names
//! the acting identity (default "unknown").

use std::env;

use application::audit::{AppendAuditEvent, ShowAuditLog, VerifyAuditChain};
use domain::audit::{ChainVerification, ChainedEvent};
use infrastructure::{FileAuditLog, RingSha256Hasher, SystemClock};

use crate::cli::AuditAction;

/// Environment variable naming the audit log file.
const LOG_PATH_VAR: &str = "AUDIT_LOG_PATH";

/// Log file used when the environment does not name one.
const DEFAULT_LOG_PATH: &str = "audit-log.jsonl";

/// Environment variable naming the acting identity.
const ACTOR_VAR: &str = "USER";

/// Actor recorded when the environment does not name one.
const UNKNOWN_ACTOR: &str = "unknown";

/// Dispatches an `audit` subcommand to its handler.
///
/// The read-only subcommands refuse to run against a log file that does
/// not exist: reporting a mistyped path or a deleted log as a valid empty
/// chain would defeat the point of verification. Only `append` may create
/// the file, on first use.
pub fn run(action: AuditAction, json: bool) -> anyhow::Result<()> {
    let path = log_path();
    let log = FileAuditLog::new(path.as_str(), RingSha256Hasher::new());
    match action {
        AuditAction::Append { action, resource } => append(log, &action, &resource, json),
        AuditAction::VerifyChain => {
            require_existing_log(&path)?;
            verify_chain(log, json)
        }
        AuditAction::Show => {
            require_existing_log(&path)?;
            show(log, json)
        }
    }
}

fn log_path() -> String {
    env::var(LOG_PATH_VAR).unwrap_or_else(|_| DEFAULT_LOG_PATH.to_string())
}

/// Fails when no file exists at `path`, naming the path.
///
/// An existing empty file is fine: it is a valid empty log. Only the
/// complete absence of the file is rejected here, so the distinction
/// between "nothing was ever logged" and "the log is not where the
/// configuration points" stays visible to the operator.
fn require_existing_log(path: &str) -> anyhow::Result<()> {
    if !std::path::Path::new(path).exists() {
        anyhow::bail!("audit log not found at {path}");
    }
    Ok(())
}

fn append(
    log: FileAuditLog<RingSha256Hasher>,
    action: &str,
    resource: &str,
    json: bool,
) -> anyhow::Result<()> {
    let actor = env::var(ACTOR_VAR).unwrap_or_else(|_| UNKNOWN_ACTOR.to_string());
    let mut use_case = AppendAuditEvent::new(log, SystemClock::new());
    let entry = use_case.execute(&actor, action, resource)?;
    if json {
        println!("{}", entry_json(&entry)?);
    } else {
        println!(
            "appended entry {} with chain {}",
            entry.event.sequence,
            entry.chain.to_hex()
        );
    }
    Ok(())
}

fn verify_chain(log: FileAuditLog<RingSha256Hasher>, json: bool) -> anyhow::Result<()> {
    let use_case = VerifyAuditChain::new(log, RingSha256Hasher::new());
    match use_case.execute()? {
        ChainVerification::Valid { entries } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "valid": true, "entries": entries })
                );
            } else {
                println!("audit chain valid ({entries} entries)");
            }
            Ok(())
        }
        ChainVerification::Broken { first_broken_index } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "valid": false,
                        "first_broken_index": first_broken_index,
                    })
                );
            }
            anyhow::bail!("audit chain broken at index {first_broken_index}");
        }
    }
}

fn show(log: FileAuditLog<RingSha256Hasher>, json: bool) -> anyhow::Result<()> {
    let entries = ShowAuditLog::new(log).execute()?;
    if json {
        let body: Vec<serde_json::Value> = entries
            .iter()
            .map(entry_json)
            .collect::<anyhow::Result<_>>()?;
        println!("{}", serde_json::Value::Array(body));
    } else {
        for entry in &entries {
            println!(
                "{} {} {} {} {} {}",
                entry.event.sequence,
                entry.event.timestamp_rfc3339()?,
                entry.event.actor,
                entry.event.action,
                entry.event.resource,
                entry.chain.to_hex()
            );
        }
    }
    Ok(())
}

fn entry_json(entry: &ChainedEvent) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "sequence": entry.event.sequence,
        "timestamp": entry.event.timestamp_rfc3339()?,
        "actor": entry.event.actor,
        "action": entry.event.action,
        "resource": entry.event.resource,
        "chain": entry.chain.to_hex(),
    }))
}
