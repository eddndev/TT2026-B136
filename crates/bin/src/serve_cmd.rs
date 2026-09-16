//! Composition root for the local HTTP case, document and participant application.

use std::fs;
use std::sync::Arc;

use anyhow::Context;
use application::case_stages::CaseStageService;
use application::cases::CaseService;
use application::documents::{
    CaseDocumentService, DocumentProcessor, DocumentProcessorPorts, EvidenceMaterial,
};
use application::hearings::HearingService;
use application::identity::{IdentityPorts, IdentityService, IdentityWorkflow};
use application::participants::ParticipantService;
use application::typed_participants::TypedParticipantService;
use infrastructure::case_stages::PostgresCaseStageStore;
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::{
    openssl_version, AesGcmSecretProtector, Argon2idHasher, EnvelopeKeyManager, LocalOpensslTsa,
    PostgresAuditLog, PostgresCaseDocumentStore, PostgresCaseRepository, PostgresHearingStore,
    PostgresParticipantStore, PostgresTypedParticipantStore, PostgresUserRepository,
    RandomRecoveryCodeGenerator, RedisSessionStore, Rfc3161Verifier, RingAesGcmCipher,
    RingSha256Hasher, RsaPkcs1Signer, RsaPkcs1Verifier, StoredZipWriter, SystemClock,
    TotpRsProvider, X509ChainValidator,
};
use zeroize::Zeroizing;

use crate::cli::ServeArgs;
use crate::vault_cmd::load_kek;

const KEK_VAR: &str = "KEK_BASE64";

/// Builds all local adapters and blocks while the HTTP server is running.
pub fn run(args: &ServeArgs) -> anyhow::Result<()> {
    let format_validator = infrastructure::document_formats::IsolatedDocumentFormatValidator::new(
        std::env::current_exe().context("cannot locate document validation worker")?,
        fs::canonicalize(&args.qpdf_library).context("cannot locate native qpdf library")?,
    )
    .context("cannot configure document format validation")?;
    format_validator
        .check_configuration()
        .context("document format startup check failed")?;
    let signer_certificate = read(&args.signer_cert, "signer certificate")?;
    let signer_key = Zeroizing::new(read(&args.signer_key, "signer private key")?);
    let issuer_certificate = read(&args.ca_cert, "issuer certificate")?;
    let crl = read(&args.crl, "certificate revocation list")?;
    let tsa_chain_path = args.tsa_dir.join("tsa-chain.pem");
    let tsa_chain = read(&tsa_chain_path, "timestamp authority chain")?;
    let kek = load_kek(KEK_VAR)?;
    let database_url = required_env("DATABASE_URL")?;
    let redis_url = required_env("REDIS_URL")?;

    infrastructure::legacy::require_completed_import(&args.data_dir, &database_url)
        .context("legacy storage has not completed database cutover")?;
    let repository = Arc::new(
        PostgresCaseDocumentStore::open(&database_url)
            .context("cannot open PostgreSQL document store")?,
    );
    let signer = RsaPkcs1Signer::new(signer_key).context("cannot load signer private key")?;
    let ports = DocumentProcessorPorts {
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
    let case_repository = Arc::new(
        PostgresCaseRepository::open(&database_url, Arc::new(RingSha256Hasher::new()))
            .context("cannot initialize PostgreSQL case repository")?,
    );
    let identity: Arc<dyn IdentityWorkflow> = Arc::new(IdentityService::new(IdentityPorts {
        users: Arc::new(
            PostgresUserRepository::open(&database_url)
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
        audit_log: Box::new(
            PostgresAuditLog::open(&database_url).context("cannot open PostgreSQL audit log")?,
        ),
    }));
    let cases = CaseService::new(
        case_repository,
        identity.clone(),
        Arc::new(SystemClock::new()),
    );
    let participants = ParticipantService::new(
        Arc::new(
            PostgresParticipantStore::open(&database_url, Arc::new(RingSha256Hasher::new()))
                .context("cannot open PostgreSQL participant store")?,
        ),
        identity.clone(),
        Arc::new(SystemClock::new()),
    );
    let processor = Arc::new(
        DocumentProcessor::new(ports, material, kek)
            .context("cannot initialize document cryptography")?,
    );
    let format_validator = Arc::new(format_validator);
    let stages = CaseStageService::new(
        Arc::new(
            PostgresCaseStageStore::open(&database_url, Arc::new(RingSha256Hasher::new()))
                .context("cannot open PostgreSQL case stage store")?,
        ),
        identity.clone(),
        processor.clone(),
        format_validator.clone(),
        Arc::new(SystemClock::new()),
    );
    let typed_participants = TypedParticipantService::new(
        identity.clone(),
        Arc::new(
            PostgresTypedParticipantStore::open(
                &database_url,
                Arc::new(RingSha256Hasher::new()),
                Arc::new(SystemClock::new()),
            )
            .context("cannot open PostgreSQL typed participant store")?,
        ),
        processor.clone(),
        Arc::new(RingSha256Hasher::new()),
        format_validator.clone(),
        Arc::new(InternalRsaDeclarationVerifier::new()),
        Arc::new(SystemClock::new()),
    );
    let hearing_hasher = Arc::new(RingSha256Hasher::new());
    let hearing_clock = Arc::new(SystemClock::new());
    let hearings = HearingService::new(
        Arc::new(
            PostgresHearingStore::open(
                &database_url,
                hearing_hasher.clone(),
                hearing_clock.clone(),
            )
            .context("cannot open PostgreSQL hearing store")?,
        ),
        identity.clone(),
        processor.clone(),
        format_validator,
        hearing_hasher,
        hearing_clock,
    );
    let workflow = CaseDocumentService::new(
        repository,
        identity.clone(),
        processor,
        Arc::new(SystemClock::new()),
    );
    let router = web::api_router(
        Arc::new(workflow),
        identity,
        web::CaseWorkflows {
            cases: Arc::new(cases),
            participants: Arc::new(participants),
            stages: Arc::new(stages),
            typed: Arc::new(typed_participants),
            hearings: Arc::new(hearings),
        },
        web::HttpLimits {
            max_requests: args.max_in_flight_requests,
            max_blocking_operations: args.max_blocking_operations,
        },
    );

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
