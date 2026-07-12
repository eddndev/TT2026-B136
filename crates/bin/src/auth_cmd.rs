//! Handlers for the `auth` command group.
//!
//! Each handler wires the concrete adapters into one use case and formats
//! its output; no domain logic lives here. Passwords arrive on standard
//! input and stay in wiped-on-drop buffers. The enrollment handler prints
//! secret material because that is the command's explicit purpose.

use std::fs;
use std::io::Write;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use application::auth::{
    CalibratePasswordHashing, CalibrationVerdict, EnrollTotp, HashPassword, VerifyPassword,
    VerifyTotp, DEFAULT_BAND_MAX_MS, DEFAULT_BAND_MIN_MS,
};
use domain::crypto::password::PasswordVerification;
use domain::crypto::totp::TotpVerification;
use infrastructure::{
    decode_base32_secret, Argon2idHasher, RandomRecoveryCodeGenerator, TotpRsProvider,
};
use zeroize::Zeroizing;

use crate::cli::{AuthAction, TotpAction};

/// Number of hash operations measured by `auth calibrate`.
const CALIBRATION_RUNS: u32 = 5;

/// Dispatches one `auth` subcommand.
pub fn run(action: AuthAction) -> anyhow::Result<()> {
    match action {
        AuthAction::Calibrate => calibrate(),
        AuthAction::HashPassword => hash_password(),
        AuthAction::VerifyPassword { hash } => verify_password(&hash),
        AuthAction::Totp { action } => match action {
            TotpAction::Enroll { user, secret_out } => totp_enroll(&user, &secret_out),
            TotpAction::Verify { code, secret_file } => totp_verify(&code, &secret_file),
        },
    }
}

/// Reads one line from standard input into a wiped-on-drop buffer,
/// stripping the trailing newline.
fn read_password_from_stdin() -> anyhow::Result<Zeroizing<String>> {
    let mut buffer = Zeroizing::new(String::new());
    std::io::stdin().read_line(&mut buffer)?;
    while buffer.ends_with('\n') || buffer.ends_with('\r') {
        buffer.pop();
    }
    if buffer.is_empty() {
        anyhow::bail!("expected the password on standard input");
    }
    Ok(buffer)
}

fn hash_password() -> anyhow::Result<()> {
    let password = read_password_from_stdin()?;
    let phc = HashPassword::new(Argon2idHasher::new()).execute(&password)?;
    println!("{phc}");
    Ok(())
}

fn verify_password(stored_phc: &str) -> anyhow::Result<()> {
    let password = read_password_from_stdin()?;
    match VerifyPassword::new(Argon2idHasher::new()).execute(&password, stored_phc)? {
        PasswordVerification::Match => println!("match"),
        PasswordVerification::Mismatch => println!("mismatch"),
    }
    Ok(())
}

fn calibrate() -> anyhow::Result<()> {
    let runs = NonZeroU32::new(CALIBRATION_RUNS).expect("the run count is a nonzero constant");
    let report = CalibratePasswordHashing::new(Argon2idHasher::new()).execute(
        runs,
        DEFAULT_BAND_MIN_MS,
        DEFAULT_BAND_MAX_MS,
    )?;
    let position = match report.verdict {
        CalibrationVerdict::BelowBand => "below",
        CalibrationVerdict::InBand => "inside",
        CalibrationVerdict::AboveBand => "above",
    };
    println!(
        "mean over {} runs: {:.1} ms ({} the {}-{} ms target band)",
        report.runs, report.mean_ms, position, report.band_min_ms, report.band_max_ms
    );
    Ok(())
}

fn totp_enroll(user: &str, secret_out: &Path) -> anyhow::Result<()> {
    let use_case = EnrollTotp::new(
        TotpRsProvider::new(),
        RandomRecoveryCodeGenerator,
        Argon2idHasher::new(),
    );
    let bundle = use_case.execute(user)?;
    write_secret_file(secret_out, &bundle.enrollment.secret_base32)?;
    println!("otpauth-uri: {}", bundle.enrollment.otpauth_uri.as_str());
    println!(
        "secret-base32: {}",
        bundle.enrollment.secret_base32.as_str()
    );
    println!("recovery codes (each works exactly once):");
    for code in &bundle.plain_recovery_codes {
        println!("  {}", code.as_str());
    }
    println!("secret written to {}", secret_out.display());
    Ok(())
}

/// Writes the base32 secret, creating the file owner-readable only.
fn write_secret_file(path: &Path, secret_base32: &str) -> anyhow::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(secret_base32.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

fn totp_verify(code: &str, secret_file: &Path) -> anyhow::Result<()> {
    let stored = Zeroizing::new(fs::read_to_string(secret_file)?);
    let secret = decode_base32_secret(&stored)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    match VerifyTotp::new(TotpRsProvider::new()).execute(&secret, code.trim(), now)? {
        TotpVerification::Accepted => println!("accepted"),
        TotpVerification::Rejected => println!("rejected"),
    }
    Ok(())
}
