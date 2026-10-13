//! Initial CA enrollment is local, private, MFA-confirmed and separately persisted.
#![cfg(target_os = "linux")]

mod ca_enrollment_support;
#[allow(dead_code)]
#[path = "../../infrastructure/tests/ca_identity_cases/support.rs"]
mod ca_identity_support;

use std::{
    fs,
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt},
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

use application::identity::{SecretProtector, UserRepository};
use domain::crypto::{PasswordHasher, PasswordVerification, RECOVERY_CODE_COUNT};
use infrastructure::{decode_base32_secret, AesGcmSecretProtector, Argon2idHasher};
use serde_json::{json, Value};

use ca_enrollment_support::{private_json, totp, Harness};

#[test]
fn private_bundle_and_totp_confirmation_precede_atomic_initial_enrollment() {
    let mut harness = Harness::new();
    let mut child = harness.spawn();
    child.line(&harness.password);
    child.line(&harness.password);
    let bundle = harness.bundle(&mut child);
    harness.assert_empty();
    let metadata = fs::metadata(&harness.enrollment).unwrap();
    assert_eq!(metadata.mode() & 0o777, 0o600);
    assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
    assert_eq!(bundle["operator_email"], harness.db.operator.email);
    assert_eq!(
        bundle["recovery_codes"].as_array().unwrap().len(),
        RECOVERY_CODE_COUNT
    );
    assert!(bundle.get("password").is_none());
    assert!(bundle.get("kek_base64").is_none());
    child.line(&totp(&bundle));
    let result = child.finish();
    assert!(result.status.success(), "confirmed CA enrollment failed");
    harness.assert_private_output(&result, Some(&bundle));
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        receipt,
        json!({"operator_id":bundle["operator_id"],
        "operator_email":bundle["operator_email"]})
    );
    let (users, _audit) = harness.db.open().unwrap();
    let stored = users
        .find_by_email(&harness.db.operator.email)
        .unwrap()
        .unwrap();
    assert_eq!(stored.id.to_string(), bundle["operator_id"]);
    assert_eq!(
        Argon2idHasher::new()
            .verify(&harness.password, &stored.password_hash)
            .unwrap(),
        PasswordVerification::Match
    );
    let secret = AesGcmSecretProtector::new(harness.kek.clone())
        .unwrap()
        .expose(stored.id, &stored.protected_totp_secret)
        .unwrap();
    let expected = decode_base32_secret(bundle["totp_secret_base32"].as_str().unwrap()).unwrap();
    assert!(
        *secret == *expected,
        "stored TOTP secret differs from its private delivery"
    );
    let row = harness
        .db
        .admin()
        .query_one(
            "SELECT (SELECT count(*) FROM qadra_ca.users),
                (SELECT count(*) FROM qadra_ca.audit_events WHERE action='ca.operator_enrolled')",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 1);
    assert_eq!(row.get::<_, i64>(1), 1);
    harness.db.assert_office_unchanged();
    harness.enrollment = harness.directory.path().join("second-enrollment.json");
    let repeated = harness.spawn().finish();
    assert!(!repeated.status.success());
    assert!(!harness.enrollment.exists());
}

#[test]
fn wrong_totp_does_not_insert_a_ca_account_or_print_private_material() {
    let mut harness = Harness::new();
    let mut child = harness.spawn();
    child.line(&harness.password);
    child.line(&harness.password);
    let bundle = harness.bundle(&mut child);
    child.line("invalid");
    let result = child.finish();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    harness.assert_private_output(&result, Some(&bundle));
    harness.assert_empty();
}

#[test]
fn a_changed_delivery_file_cannot_be_used_to_confirm_an_enrollment() {
    let mut harness = Harness::new();
    let mut child = harness.spawn();
    child.line(&harness.password);
    child.line(&harness.password);
    let bundle = harness.bundle(&mut child);
    fs::write(&harness.enrollment, b"{}").unwrap();
    child.line(&totp(&bundle));
    let result = child.finish();
    assert!(!result.status.success());
    harness.assert_private_output(&result, Some(&bundle));
    harness.assert_empty();
}

#[test]
fn password_mismatch_and_end_of_input_leave_no_account() {
    let mut harness = Harness::new();
    for (index, lines) in [
        vec![harness.password.to_string()],
        vec![harness.password.to_string(), "different-password".into()],
    ]
    .into_iter()
    .enumerate()
    {
        harness.enrollment = harness
            .directory
            .path()
            .join(format!("incomplete-{index}.json"));
        let mut child = harness.spawn();
        for line in &lines {
            child.line(line);
        }
        let result = child.finish();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        harness.assert_private_output(&result, None);
        harness.assert_empty();
    }
}

#[test]
fn existing_or_symlink_delivery_and_invalid_setup_files_are_rejected_before_enrollment() {
    let mut harness = Harness::new();
    private_json(&harness.enrollment, &json!({"keep":"existing"}));
    let before = fs::read(&harness.enrollment).unwrap();
    assert!(!harness.spawn().finish().status.success());
    assert!(fs::read(&harness.enrollment).unwrap() == before);
    let original = harness.enrollment.clone();
    harness.enrollment = harness.directory.path().join("linked-delivery.json");
    std::os::unix::fs::symlink(&original, &harness.enrollment).unwrap();
    assert!(!harness.spawn().finish().status.success());
    assert!(fs::read(&original).unwrap() == before);
    harness.enrollment = harness.directory.path().join("new-delivery.json");
    fs::set_permissions(&harness.setup, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!harness.spawn().finish().status.success());
    assert!(!harness.enrollment.exists());
    let mut config = harness.config.clone();
    config["operator_id"] = json!(uuid::Uuid::new_v4());
    harness.rewrite_config(&config);
    assert!(!harness.spawn().finish().status.success());
    assert!(!harness.enrollment.exists());
    harness.assert_empty();
}

#[test]
fn terminal_entry_disables_echo_and_restores_it_when_the_operator_cancels() {
    use rustix::{
        pty::{grantpt, openpt, ptsname, unlockpt, OpenptFlags},
        termios::{tcgetattr, LocalModes},
    };
    let mut harness = Harness::new();
    let mut master = fs::File::from(
        openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).unwrap(),
    );
    grantpt(&master).unwrap();
    unlockpt(&master).unwrap();
    let name = ptsname(&master, Vec::new()).unwrap();
    let slave = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(name.to_str().unwrap())
        .unwrap();
    let initial = tcgetattr(&slave).unwrap();
    assert!(initial.local_modes.contains(LocalModes::ECHO));
    let mut child = harness.spawn_with_input(Stdio::from(slave.try_clone().unwrap()));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            child.child.try_wait().unwrap().is_none(),
            "CA enrollment exited before private input"
        );
        if !tcgetattr(&slave)
            .unwrap()
            .local_modes
            .contains(LocalModes::ECHO)
        {
            break;
        }
        assert!(Instant::now() < deadline, "terminal echo was not disabled");
        thread::sleep(Duration::from_millis(20));
    }
    master.write_all(&[3]).unwrap();
    let result = child.finish();
    assert!(!result.status.success());
    assert_eq!(tcgetattr(&slave).unwrap().local_modes, initial.local_modes);
    harness.assert_empty();
}
