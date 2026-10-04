use application::credential_trust::{
    CredentialTrustExpectation, CredentialTrustPublisher, CredentialTrustService,
    CredentialTrustSnapshot,
};
use infrastructure::{
    certificates::InternalRsaDeclarationVerifier, PostgresCredentialTrustStore, SystemClock,
};
use std::{fs, path::Path, process::Command, sync::Arc};
use tempfile::TempDir;

pub struct Material {
    directory: TempDir,
    pub leaf: Vec<u8>,
}

impl Material {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().expect("private Owner login material directory");
        script(directory.path(), "init-ca.sh", &[]);
        script(directory.path(), "issue-cert.sh", &["Native Owner Login"]);
        script(directory.path(), "gen-crl.sh", &[]);
        let leaf = fs::read(directory.path().join("certs/native-owner-login.crt.pem"))
            .expect("read public Owner Partner certificate");
        Self { directory, leaf }
    }

    pub fn publish(
        &self,
        database: &str,
        expected: CredentialTrustExpectation,
    ) -> CredentialTrustSnapshot {
        let clock = Arc::new(SystemClock::new());
        let store = PostgresCredentialTrustStore::open(database, clock.clone())
            .unwrap_or_else(|_| panic!("open actual trust publisher"));
        let publisher = CredentialTrustService::new(
            Arc::new(store),
            Arc::new(InternalRsaDeclarationVerifier::new()),
            clock,
        );
        let root =
            fs::read(self.directory.path().join("ca.crt.pem")).expect("read public trust root");
        let crl = fs::read(self.directory.path().join("crl/crl.pem")).expect("read public CRL");
        publisher
            .publish(&root, &crl, expected)
            .unwrap_or_else(|_| panic!("publish actual current credential trust"))
    }

    pub fn rotate(
        &self,
        database: &str,
        prior: &CredentialTrustSnapshot,
    ) -> CredentialTrustSnapshot {
        script(self.directory.path(), "gen-crl.sh", &[]);
        let next = self.publish(
            database,
            CredentialTrustExpectation::Revision(prior.revision),
        );
        assert_eq!(next.deployment_id, prior.deployment_id);
        assert_eq!(next.revision.get(), prior.revision.get() + 1);
        assert_eq!(
            next.inspection.root_fingerprint,
            prior.inspection.root_fingerprint
        );
        assert!(next.inspection.crl_number > prior.inspection.crl_number);
        assert_ne!(next.inspection.crl_digest, prior.inspection.crl_digest);
        next
    }

    pub fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        let statement = self.directory.path().join("statement.bin");
        fs::write(&statement, bytes).expect("write public statement for external signer");
        let result = Command::new("openssl")
            .args(["dgst", "-sha256", "-sign"])
            .arg(
                self.directory
                    .path()
                    .join("private/native-owner-login.key.pem"),
            )
            .arg(statement)
            .output()
            .expect("start external RSA signer");
        assert!(result.status.success(), "external RSA signer failed");
        assert_eq!(
            result.stdout.len(),
            384,
            "external signature must be RSA-3072"
        );
        result.stdout
    }
}

fn script(directory: &Path, name: &str, arguments: &[&str]) {
    let result = Command::new("bash")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../pki")
                .join(name),
        )
        .args(arguments)
        .env("PKI_CA_DIR", directory)
        .output()
        .expect("start private PKI fixture script");
    assert!(result.status.success(), "private PKI fixture script failed");
}
