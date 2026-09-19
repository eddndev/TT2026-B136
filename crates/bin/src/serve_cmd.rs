//! Composition root for the local HTTP case, document and participant application.

use std::fs;
use std::sync::Arc;

use anyhow::Context;
use application::case_stages::CaseStageService;
use application::cases::CaseService;
use application::deadline_profiles::DeadlineProfileService;
use application::deadlines::DeadlineService;
use application::documents::{
    CaseDocumentService, DocumentProcessor, DocumentProcessorPorts, EvidenceMaterial,
};
use application::hearing_results::HearingResultService;
use application::hearings::HearingService;
use application::identity::{IdentityPorts, IdentityService, IdentityWorkflow};
use application::judicial_calendars::JudicialCalendarService;
use application::participants::ParticipantService;
use application::procedural_facts::ProceduralFactService;
use application::typed_participants::TypedParticipantService;
use infrastructure::case_stages::PostgresCaseStageStore;
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::{
    openssl_version, AesGcmSecretProtector, Argon2idHasher, EnvelopeKeyManager, LocalOpensslTsa,
    PostgresAuditLog, PostgresCaseDocumentStore, PostgresCaseRepository,
    PostgresHearingResultStore, PostgresHearingStore, PostgresJudicialCalendarStore,
    PostgresParticipantStore, PostgresTypedParticipantStore, PostgresUserRepository,
    RandomRecoveryCodeGenerator, RedisSessionStore, Rfc3161Verifier, RingAesGcmCipher,
    RingSha256Hasher, RsaPkcs1Signer, RsaPkcs1Verifier, StoredZipWriter, SystemClock,
    TotpRsProvider, X509ChainValidator,
};
use infrastructure::{PostgresDeadlineProfileStore, PostgresProceduralFactStore};
use zeroize::Zeroizing;

use crate::cli::ServeArgs;
use crate::serve_deadline_runtime::DeadlineRuntimeConfig;
use crate::vault_cmd::load_kek;

const KEK_VAR: &str = "KEK_BASE64";

/// Builds all local adapters and blocks while the HTTP server is running.
pub fn run(args: &ServeArgs) -> anyhow::Result<()> {
    let alert_email = crate::serve_alert_composition::email_settings(args)?;
    let alert_config = crate::serve_alert_runtime::AlertRuntimeConfig::new(
        args.alert_page_limit,
        std::time::Duration::from_millis(u64::from(args.alert_poll_ms.get())),
    )?;
    let deadline_config = DeadlineRuntimeConfig::new(
        application::deadline_dispatch::DeadlineDispatchLimit::new(args.deadline_page_limit)?,
        std::time::Duration::from_millis(u64::from(args.deadline_poll_ms.get())),
    )?;
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
        format_validator.clone(),
        hearing_hasher,
        hearing_clock,
    );
    let result_hasher = Arc::new(RingSha256Hasher::new());
    let result_clock = Arc::new(SystemClock::new());
    let hearing_results = HearingResultService::new(
        Arc::new(
            PostgresHearingResultStore::open(
                &database_url,
                result_hasher.clone(),
                result_clock.clone(),
            )
            .context("cannot open PostgreSQL hearing result store")?,
        ),
        identity.clone(),
        processor.clone(),
        format_validator.clone(),
        result_hasher,
        result_clock,
    );
    let fact_hasher = Arc::new(RingSha256Hasher::new());
    let fact_clock = Arc::new(SystemClock::new());
    let procedural_facts = ProceduralFactService::new(
        Arc::new(
            PostgresProceduralFactStore::open(
                &database_url,
                fact_hasher.clone(),
                fact_clock.clone(),
            )
            .context("cannot open PostgreSQL procedural fact store")?,
        ),
        identity.clone(),
        processor.clone(),
        format_validator,
        fact_hasher,
        fact_clock,
    );
    let calendar_hasher = Arc::new(RingSha256Hasher::new());
    let calendar_clock = Arc::new(SystemClock::new());
    let calendars = JudicialCalendarService::new(
        Arc::new(
            PostgresJudicialCalendarStore::open(
                &database_url,
                calendar_hasher.clone(),
                calendar_clock.clone(),
            )
            .context("cannot open PostgreSQL judicial calendar store")?,
        ),
        identity.clone(),
        calendar_hasher,
        calendar_clock,
    );
    let profile_hasher = Arc::new(RingSha256Hasher::new());
    let profile_clock = Arc::new(SystemClock::new());
    let profiles = DeadlineProfileService::new(
        Arc::new(
            PostgresDeadlineProfileStore::open(
                &database_url,
                profile_hasher.clone(),
                profile_clock.clone(),
            )
            .context("cannot open PostgreSQL deadline profile store")?,
        ),
        identity.clone(),
        profile_hasher,
        profile_clock,
    );
    let deadline_hasher = Arc::new(RingSha256Hasher::new());
    let deadline_clock = Arc::new(SystemClock::new());
    let dispatch = infrastructure::PostgresDeadlineDispatchStore::open(
        &database_url,
        deadline_hasher.clone(),
        deadline_clock.clone(),
    )
    .context("cannot open PostgreSQL deadline dispatcher")?;
    let worker = infrastructure::PostgresDeadlineWorkerStore::open(
        &database_url,
        deadline_hasher.clone(),
        deadline_clock.clone(),
    )
    .context("cannot open PostgreSQL deadline worker")?;
    let agenda = application::agenda::AgendaService::new(
        Arc::new(
            infrastructure::PostgresAgendaStore::open(
                &database_url,
                deadline_hasher.clone(),
                deadline_clock.clone(),
            )
            .context("cannot open PostgreSQL agenda store")?,
        ),
        identity.clone(),
    );
    let deadlines = DeadlineService::new(
        Arc::new(
            infrastructure::PostgresDeadlineStore::open(
                &database_url,
                deadline_hasher.clone(),
                deadline_clock.clone(),
            )
            .context("cannot open PostgreSQL deadline store")?,
        ),
        identity.clone(),
        deadline_hasher,
        deadline_clock,
    );
    let workflow = CaseDocumentService::new(
        repository,
        identity.clone(),
        processor,
        Arc::new(SystemClock::new()),
    );
    let alerts = crate::serve_alert_composition::open(
        &database_url,
        identity.clone(),
        alert_email,
        alert_config,
    )?;
    let router = web::api_router(
        Arc::new(workflow),
        identity,
        web::CaseWorkflows {
            cases: Arc::new(cases),
            participants: Arc::new(participants),
            stages: Arc::new(stages),
            typed: Arc::new(typed_participants),
            hearings: Arc::new(hearings),
            hearing_results: Arc::new(hearing_results),
            procedural_facts: Arc::new(procedural_facts),
            deadlines: Arc::new(deadlines),
            agenda: Arc::new(agenda),
            alerts: alerts.workflow,
        },
        Arc::new(calendars),
        Arc::new(profiles),
        web::HttpLimits {
            max_requests: args.max_in_flight_requests,
            max_blocking_operations: args.max_blocking_operations,
        },
    );

    crate::serve_start::run(
        &args.bind,
        router,
        dispatch,
        worker,
        deadline_config,
        alerts.consumers,
    )
}

fn read(path: &std::path::Path, label: &str) -> anyhow::Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("cannot read {label} at {}", path.display()))
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set for the HTTP application"))
}
