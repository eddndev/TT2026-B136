//! End-to-end checks of `package export` through the compiled binary:
//! the produced archive is extracted with the system unzip binary and
//! then verified by parsing the shell commands out of the generated
//! INSTRUCCIONES.md and running exactly those, including one corruption
//! case that those commands must detect, plus the refusal of artifact
//! sets that do not verify against each other.

mod package_fixture;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use package_fixture::{exe, pki_dir, run_checked, Fixture};

/// The shell commands the instructions document: the four-space
/// indented code lines that invoke a tool, in the order a reader would
/// run them. Expected outputs, such as the recorded digest, are also
/// indented but do not start with a tool name.
fn documented_commands(instructions: &str) -> Vec<String> {
    instructions
        .lines()
        .filter_map(|line| line.strip_prefix("    "))
        .map(str::trim_end)
        .filter(|code| code.starts_with("openssl ") || code.starts_with("cat "))
        .map(str::to_string)
        .collect()
}

/// Runs one documented command inside `dir`, mirroring how a third
/// party would follow the instructions, and returns its output.
fn documented_step(dir: &Path, script: &str) -> Output {
    Command::new("bash")
        .arg("-ec")
        .arg(script)
        .current_dir(dir)
        .output()
        .expect("bash is runnable")
}

fn assert_step_ok(dir: &Path, script: &str) -> Output {
    let output = documented_step(dir, script);
    assert!(
        output.status.success(),
        "documented command failed: {script}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Standard output and standard error of one step, concatenated, so
/// expected phrases are found no matter which stream openssl uses.
fn text_of(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn the_documented_openssl_commands_verify_the_extracted_package() {
    let fixture = Fixture::build();
    let extracted = fixture.export_and_extract();

    let instructions = fs::read_to_string(extracted.join("INSTRUCCIONES.md"))
        .expect("the package carries INSTRUCCIONES.md");
    let commands = documented_commands(&instructions);
    assert_eq!(
        commands.len(),
        7,
        "the instructions document seven commands: {commands:#?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| command.starts_with("openssl ts -verify")
                && command.contains("-CAfile tsa-chain.pem")),
        "with a bundled tsa chain the token check must anchor on it: {commands:#?}"
    );

    // Run every documented command in order, exactly as written.
    let outputs: Vec<Output> = commands
        .iter()
        .map(|command| assert_step_ok(&extracted, command))
        .collect();

    // The digest the instructions record matches the fresh computation
    // of the first documented command.
    let digest_line = String::from_utf8_lossy(&outputs[0].stdout).into_owned();
    let fresh_digest = digest_line
        .rsplit(' ')
        .next()
        .expect("openssl dgst prints the digest last")
        .trim()
        .to_string();
    assert!(
        instructions.contains(&fresh_digest),
        "the instructions must record the digest {fresh_digest}"
    );

    // Each verification command must also report the outcome the
    // instructions describe as the expected output.
    let mut checked = 0;
    for (command, output) in commands.iter().zip(&outputs) {
        let text = text_of(output);
        if command.contains("dgst") && command.contains("-verify") {
            assert!(
                text.contains("Verified OK"),
                "signature check output: {text}"
            );
            checked += 1;
        } else if command.starts_with("openssl verify ") {
            assert!(
                text.contains("certificado.pem: OK"),
                "certificate check output: {text}"
            );
            checked += 1;
        } else if command.starts_with("openssl ts -verify") {
            assert!(
                text.contains("Verification: OK"),
                "token check output: {text}"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked, 3,
        "the signature, certificate, and token checks must all run: {commands:#?}"
    );
}

#[test]
fn the_package_members_survive_the_round_trip_byte_for_byte() {
    let fixture = Fixture::build();
    let extracted = fixture.export_and_extract();

    let expectations = [
        ("acta.txt", &fixture.document),
        ("acta.txt.sig", &fixture.signature),
        ("acta.txt.tsr", &fixture.token),
        ("certificado.pem", &fixture.certificate),
        ("ca.pem", &fixture.root),
        ("crl.pem", &fixture.crl),
        ("tsa-chain.pem", &fixture.tsa_chain),
    ];
    for (member, original) in expectations {
        assert_eq!(
            fs::read(extracted.join(member)).expect(member),
            fs::read(original).unwrap(),
            "{member} must match its source byte for byte"
        );
    }
}

#[test]
fn a_corrupted_member_is_caught_by_the_documented_commands() {
    let fixture = Fixture::build();
    let extracted = fixture.export_and_extract();
    let instructions = fs::read_to_string(extracted.join("INSTRUCCIONES.md"))
        .expect("the package carries INSTRUCCIONES.md");
    let commands = documented_commands(&instructions);

    // Corrupt the extracted document the way a post-extraction tamper
    // would: one flipped byte.
    let member = extracted.join("acta.txt");
    let mut bytes = fs::read(&member).unwrap();
    bytes[5] ^= 0xff;
    fs::write(&member, bytes).unwrap();

    // Following the instructions in order, the signature check and the
    // token check must both reject the altered document.
    let mut rejections = 0;
    for command in &commands {
        let output = documented_step(&extracted, command);
        let is_signature_check = command.contains("dgst") && command.contains("-verify");
        let is_token_check = command.starts_with("openssl ts -verify");
        if is_signature_check || is_token_check {
            assert!(
                !output.status.success(),
                "the documented check must reject the altered document: {command}"
            );
            rejections += 1;
        }
    }
    assert_eq!(
        rejections, 2,
        "both document-covering checks must run: {commands:#?}"
    );
}

#[test]
fn export_refuses_a_revocation_list_from_another_authority() {
    let mut fixture = Fixture::build();
    let other_ca = fixture.dir.path().join("otra-ca");
    run_checked(
        Command::new(exe())
            .args(["pki", "--scripts-dir"])
            .arg(pki_dir())
            .arg("init-ca")
            .env("PKI_CA_DIR", &other_ca),
        "pki init-ca for the unrelated authority",
    );
    run_checked(
        Command::new(exe())
            .args(["pki", "--scripts-dir"])
            .arg(pki_dir())
            .arg("gen-crl")
            .env("PKI_CA_DIR", &other_ca),
        "pki gen-crl for the unrelated authority",
    );
    fixture.crl = other_ca.join("crl/crl.pem");

    let package = fixture.dir.path().join("evidencia.zip");
    let output = fixture.export(&package, false);
    assert!(
        !output.status.success(),
        "export must refuse a revocation list from another authority"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("revocation list"),
        "the refusal must name the revocation list: {stderr}"
    );
    assert!(!package.exists(), "no package may be written on refusal");
}

#[test]
fn export_refuses_a_tampered_signature_naming_the_component() {
    let fixture = Fixture::build();
    let mut bytes = fs::read(&fixture.signature).unwrap();
    bytes[3] ^= 0xff;
    fs::write(&fixture.signature, bytes).unwrap();

    let package = fixture.dir.path().join("evidencia.zip");
    let output = fixture.export(&package, false);
    assert!(
        !output.status.success(),
        "export must refuse a signature that does not verify"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("signature"),
        "the refusal must name the signature component: {stderr}"
    );
    assert!(!package.exists(), "no package may be written on refusal");
}

#[test]
fn export_reports_the_digest_and_package_path_as_json() {
    let fixture = Fixture::build();
    let package = fixture.dir.path().join("evidencia.zip");
    let output = fixture.export(&package, true);
    assert!(
        output.status.success(),
        "package export --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--json prints one object");

    let document_bytes = fs::read(&fixture.document).unwrap();
    let expected_digest = {
        use domain::crypto::DocumentHasher;
        infrastructure::RingSha256Hasher::new()
            .hash_bytes(&document_bytes)
            .to_hex()
    };
    assert_eq!(body["digest"], expected_digest.as_str());
    assert_eq!(body["package"], package.display().to_string());
    assert_eq!(body["instructions"], "INSTRUCCIONES.md");
    assert!(package.is_file(), "the archive must exist at --out");
}
