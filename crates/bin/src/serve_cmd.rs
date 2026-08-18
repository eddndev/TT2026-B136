//! Composition root for the local HTTP document application.

use std::fs;
use std::sync::Arc;

use anyhow::Context;
use application::documents::{DocumentWorkflowPorts, EvidenceMaterial, LocalDocumentWorkflow};
use application::identity::{IdentityPorts, IdentityService};
use infrastructure::{
    openssl_version, AesGcmSecretProtector, Argon2idHasher, EnvelopeKeyManager, FileAuditLog,
    FileDocumentRepository, LocalOpensslTsa, PostgresUserRepository, RandomRecoveryCodeGenerator,
    RedisSessionStore, Rfc3161Verifier, RingAesGcmCipher, RingSha256Hasher, RsaPkcs1Signer,
    RsaPkcs1Verifier, StoredZipWriter, SystemClock, TotpRsProvider, X509ChainValidator,
};
use zeroize::Zeroizing;

use crate::cli::ServeArgs;
use crate::vault_cmd::load_kek;

const KEK_VAR: &str = "KEK_BASE64";

/// Builds all local adapters and blocks while the HTTP server is running.
pub fn run(args: &ServeArgs) -> anyhow::Result<()> {
    let signer_certificate = read(&args.signer_cert, "signer certificate")?;
    let signer_key = Zeroizing::new(read(&args.signer_key, "signer private key")?);
    let issuer_certificate = read(&args.ca_cert, "issuer certificate")?;
    let crl = read(&args.crl, "certificate revocation list")?;
    let tsa_chain_path = args.tsa_dir.join("tsa-chain.pem");
    let tsa_chain = read(&tsa_chain_path, "timestamp authority chain")?;
    let kek = load_kek(KEK_VAR)?;
    let database_url = required_env("DATABASE_URL")?;
    let redis_url = required_env("REDIS_URL")?;

    let repository = Arc::new(
        FileDocumentRepository::new(args.data_dir.join("documents"))
            .context("cannot initialize document repository")?,
    );
    let audit_log = FileAuditLog::new(args.data_dir.join("audit.jsonl"), RingSha256Hasher::new());
    let signer = RsaPkcs1Signer::new(signer_key).context("cannot load signer private key")?;
    let ports = DocumentWorkflowPorts {
        repository,
        audit_log: Box::new(audit_log),
        clock: Box::new(SystemClock::new()),
        hasher: Box::new(RingSha256Hasher::new()),
        cipher: Box::new(RingAesGcmCipher::new()),
        keys: Box::new(EnvelopeKeyManager::new()),
        signer: Box::new(signer),
        timestamp_service: Box::new(LocalOpensslTsa::new(&args.tsa_config, &args.tsa_dir)),
        signature_verifier: Box::new(RsaPkcs1Verifier::new()),
        certificate_validator: Box::new(X509ChainValidator::new()),
        timestamp_verifier: Box::new(Rfc3161Verifier::new()),
        archiver: Box::new(StoredZipWriter::new()),
    };
    let material = EvidenceMaterial {
        signer_certificate_pem: signer_certificate,
        issuer_certificate_pem: issuer_certificate,
        crl_pem: crl,
        tsa_chain_pem: Some(tsa_chain),
        openssl_version: openssl_version().context("cannot inspect openssl version")?,
    };
    let identity = IdentityService::new(IdentityPorts {
        users: Arc::new(
            PostgresUserRepository::connect(&database_url)
                .context("cannot initialize PostgreSQL user repository")?,
        ),
        sessions: Arc::new(
            RedisSessionStore::connect(&redis_url)
                .context("cannot initialize Redis session store")?,
        ),
        passwords: Arc::new(Argon2idHasher::new()),
        totp: Arc::new(TotpRsProvider::new()),
        recovery: Arc::new(RandomRecoveryCodeGenerator),
        secrets: Arc::new(
            AesGcmSecretProtector::new(kek.clone())
                .context("cannot initialize TOTP secret protection")?,
        ),
        clock: Arc::new(SystemClock::new()),
        audit_log: Box::new(FileAuditLog::new(
            args.data_dir.join("audit.jsonl"),
            RingSha256Hasher::new(),
        )),
    });
    let workflow = LocalDocumentWorkflow::new(ports, material, kek)
        .context("cannot initialize document workflow")?;
    let router = web::application_router(Arc::new(workflow), Arc::new(identity));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("cannot initialize async runtime")?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind(&args.bind)
            .await
            .with_context(|| format!("cannot bind http server to {}", args.bind))?;
        let local_address = listener
            .local_addr()
            .context("cannot read local http address")?;
        println!("listening on http://{local_address}");
        tracing::info!(bind = %local_address, "local http application started");
        axum::serve(listener, router)
            .await
            .context("http server stopped unexpectedly")
    })
}

fn read(path: &std::path::Path, label: &str) -> anyhow::Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("cannot read {label} at {}", path.display()))
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set for the HTTP application"))
}
