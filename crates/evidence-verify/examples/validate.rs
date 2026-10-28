//! Validate one evidence package from the command line and print the report.
//!
//! Usage: `validate <package.zip> [<sha256-hex> ...]`
//!
//! Each fingerprint is the lowercase or uppercase hexadecimal SHA-256 of a
//! root certificate DER, pinned for every purpose. The current status is
//! evaluated at the local clock. The validation itself performs no I/O; only
//! this example reads the file and the clock. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use evidence_verify::policy::{PinnedRoot, Policy, Purposes};
use evidence_verify::report::{InstantSource, Report};
use evidence_verify::{validate, Outcome};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some((path, fingerprints)) = arguments.split_first() else {
        eprintln!("usage: validate <package.zip> [<sha256-hex> ...]");
        return ExitCode::from(2);
    };
    let mut pinned = Vec::new();
    for (index, text) in fingerprints.iter().enumerate() {
        let Some(sha256) = fingerprint(text) else {
            eprintln!("not a SHA-256 fingerprint in hexadecimal: {text}");
            return ExitCode::from(2);
        };
        pinned.push(PinnedRoot {
            sha256,
            purposes: Purposes::ALL,
            label: format!("pinned root {}", index + 1),
        });
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    let report = validate(&bytes, &Policy::new(pinned, now));
    print(&report);
    match report.overall {
        Outcome::Checked => ExitCode::SUCCESS,
        _ => ExitCode::from(1),
    }
}

/// The 32 bytes of a 64-digit hexadecimal fingerprint, ignoring `:`.
fn fingerprint(text: &str) -> Option<[u8; 32]> {
    let digits: Vec<u8> = text.bytes().filter(|byte| *byte != b':').collect();
    if digits.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (byte, pair) in out.iter_mut().zip(digits.chunks(2)) {
        let pair = std::str::from_utf8(pair).ok()?;
        *byte = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(out)
}

fn outcome(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Checked => "checked",
        Outcome::Failed => "failed",
        Outcome::Indeterminate => "indeterminate",
        Outcome::NotSupported => "not supported",
        Outcome::NotApplicable => "not applicable",
    }
}

fn overall(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Checked => "complete",
        Outcome::Indeterminate => "partial",
        Outcome::Failed => "failed",
        Outcome::NotSupported => "not supported",
        Outcome::NotApplicable => "not applicable",
    }
}

fn source(source: InstantSource) -> &'static str {
    match source {
        InstantSource::Token => "time-stamp token",
        InstantSource::ApplicationRecord => "application record",
        InstantSource::Caller => "local clock",
    }
}

fn print(report: &Report) {
    println!("profile: {:?}", report.profile);
    println!("overall: {}", overall(report.overall));
    println!("components:");
    for component in &report.components {
        println!("  {:?}: {}", component.kind, outcome(component.outcome));
        for finding in &component.findings {
            println!(
                "    [{}] {} {}",
                outcome(finding.outcome),
                finding.code,
                finding.detail
            );
        }
    }
    println!("instants:");
    for instant in &report.instants {
        println!(
            "  {} = {} ({})",
            instant.label,
            instant.unix,
            source(instant.source)
        );
    }
    println!("digests:");
    for (name, digest) in &report.digests {
        println!("  {digest}  {name}");
    }
    println!("trust:");
    for note in &report.trust {
        let purpose = note.purpose.map(|purpose| purpose.name()).unwrap_or("-");
        println!(
            "  {} {} {} {}",
            note.code, purpose, note.fingerprint, note.detail
        );
    }
    println!("current status:");
    for finding in &report.current_status {
        println!(
            "  [{}] {} {}",
            outcome(finding.outcome),
            finding.code,
            finding.detail
        );
    }
}
