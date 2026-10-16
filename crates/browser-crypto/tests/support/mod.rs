use browser_crypto::generate;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::OnceLock;

pub const PASSWORD: &str = "synthetic-test-password";

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub key: String,
    pub csr: String,
    pub cert: Vec<u8>,
}

pub fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let generated = generate(PASSWORD, "Browser holder", "Example Office", "MX").expect(
            "generate must return an encrypted RSA-3072 key and a verifiable PKCS#10 request",
        );
        let directory = tempfile::tempdir().unwrap();
        write_private(
            directory.path(),
            "key.pem",
            generated.encrypted_key_pem.as_bytes(),
        );
        write_private(
            directory.path(),
            "request.pem",
            generated.csr_pem.as_bytes(),
        );
        write_private(directory.path(), "password", PASSWORD.as_bytes());
        openssl(
            directory.path(),
            &[
                "x509",
                "-req",
                "-in",
                "request.pem",
                "-signkey",
                "key.pem",
                "-passin",
                "file:password",
                "-sha256",
                "-days",
                "1",
                "-outform",
                "DER",
                "-out",
                "certificate.der",
            ],
        );
        let cert = std::fs::read(directory.path().join("certificate.der")).unwrap();
        Fixture {
            directory,
            key: generated.encrypted_key_pem,
            csr: generated.csr_pem,
            cert,
        }
    })
}

pub fn write_private(directory: &Path, name: &str, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(directory.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
}

pub fn openssl_output(directory: &Path, arguments: &[&str]) -> Output {
    Command::new("openssl")
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("OpenSSL is required for independent interoperability verification")
}

pub fn openssl(directory: &Path, arguments: &[&str]) {
    let output = openssl_output(directory, arguments);
    assert!(
        output.status.success(),
        "OpenSSL rejected synthetic test material: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
