//! Composition root for the local HTTP case, document and participant application.

use std::sync::Arc;

use anyhow::Context;
use application::case_stages::CaseStageService;
use application::cases::CaseService;
use application::deadline_profiles::DeadlineProfileService;
use application::deadlines::DeadlineService;
use application::documents::CaseDocumentService;
use application::hearing_results::HearingResultService;
use application::hearings::HearingService;
use application::identity::SessionPolicy;
use application::judicial_calendars::JudicialCalendarService;
use application::participants::ParticipantService;
use application::procedural_facts::ProceduralFactService;
use application::typed_participants::TypedParticipantService;
use infrastructure::case_stages::PostgresCaseStageStore;
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::{
    PostgresCaseDocumentStore, PostgresCaseRepository, PostgresHearingResultStore,
    PostgresHearingStore, PostgresJudicialCalendarStore, PostgresParticipantStore,
    PostgresTypedParticipantStore, RingSha256Hasher, SystemClock,
};
use infrastructure::{PostgresDeadlineProfileStore, PostgresProceduralFactStore};

use crate::cli::ServeArgs;

mod documents;
mod inputs;
mod owner_certificates;
#[cfg(test)]
mod owner_login_acceptance;
use crate::serve_deadline_runtime::DeadlineRuntimeConfig;
use crate::vault_cmd::load_kek;
use inputs::required_env;

const KEK_VAR: &str = "KEK_BASE64";

/// Builds all local adapters and blocks while the HTTP server is running.
pub fn run(args: &ServeArgs) -> anyhow::Result<()> {
    let session_policy = SessionPolicy::new(
        SessionPolicy::default().absolute_ttl_seconds(),
        args.session_idle_seconds,
    )?;
    let owner_login =
        crate::serve_owner_login_config::OwnerLoginSettings::resolve(&args.owner_login)?;
    let email = crate::serve_email_credentials::load(args)?;
    let alert_config = crate::serve_alert_runtime::AlertRuntimeConfig::new(
        args.alert_page_limit,
        std::time::Duration::from_millis(u64::from(args.alert_poll_ms.get())),
    )?;
    let deadline_config = DeadlineRuntimeConfig::new(
        application::deadline_dispatch::DeadlineDispatchLimit::new(args.deadline_page_limit)?,
        std::time::Duration::from_millis(u64::from(args.deadline_poll_ms.get())),
    )?;
    let (format_validator, admission) = crate::serve_document_validation::open(args)?;
    let kek = load_kek(KEK_VAR)?;
    let processor = Arc::new(documents::open(args, kek.clone())?);
    let database_url = required_env("DATABASE_URL")?;
    let redis_url = required_env("REDIS_URL")?;

    infrastructure::legacy::require_completed_import(&args.data_dir, &database_url)
        .context("legacy storage has not completed database cutover")?;
    let stop = Arc::new(crate::serve_stop::Stop::default());
    let limits = web::HttpLimits {
        max_requests: args.max_in_flight_requests,
        max_blocking_operations: args.max_blocking_operations,
    };
    let budget = web::HttpWorkBudget::new(limits.max_blocking_operations);
    let (server, dispatch, worker, alerts, reports) =
        infrastructure::with_validated_postgres(&database_url, |database| -> anyhow::Result<_> {
            let repository = Arc::new(
                PostgresCaseDocumentStore::open(database)
                    .context("cannot open PostgreSQL document store")?,
            );
            let case_repository = Arc::new(
                PostgresCaseRepository::open(database, Arc::new(RingSha256Hasher::new()))
                    .context("cannot initialize PostgreSQL case repository")?,
            );
            let identity_components = crate::serve_identity_composition::open(
                database,
                &redis_url,
                kek.clone(),
                session_policy,
                owner_login.as_ref(),
            )?;
            let identity = identity_components.identity;
            let cases = CaseService::new(
                case_repository,
                identity.clone(),
                Arc::new(SystemClock::new()),
            );
            let participants = ParticipantService::new(
                Arc::new(
                    PostgresParticipantStore::open(database, Arc::new(RingSha256Hasher::new()))
                        .context("cannot open PostgreSQL participant store")?,
                ),
                identity.clone(),
                Arc::new(SystemClock::new()),
            );
            let reports =
                crate::serve_report_composition::open(database, identity.clone(), kek.clone())?;
            let format_validator = Arc::new(format_validator);
            let stages = CaseStageService::new(
                Arc::new(
                    PostgresCaseStageStore::open(database, Arc::new(RingSha256Hasher::new()))
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
                        database,
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
                        database,
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
                        database,
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
                        database,
                        fact_hasher.clone(),
                        fact_clock.clone(),
                    )
                    .context("cannot open PostgreSQL procedural fact store")?,
                ),
                identity.clone(),
                processor.clone(),
                format_validator.clone(),
                fact_hasher,
                fact_clock,
            );
            let resource_hasher = Arc::new(RingSha256Hasher::new());
            let resource_clock = Arc::new(SystemClock::new());
            let procedural_resources =
                application::procedural_resources::ProceduralResourceService::new(
                    Arc::new(
                        infrastructure::PostgresProceduralResourceStore::open(
                            database,
                            resource_hasher.clone(),
                            resource_clock.clone(),
                        )
                        .context("cannot open PostgreSQL procedural resource store")?,
                    ),
                    identity.clone(),
                    processor.clone(),
                    format_validator,
                    resource_hasher,
                    resource_clock,
                );
            let calendar_hasher = Arc::new(RingSha256Hasher::new());
            let calendar_clock = Arc::new(SystemClock::new());
            let calendars = JudicialCalendarService::new(
                Arc::new(
                    PostgresJudicialCalendarStore::open(
                        database,
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
                        database,
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
                database,
                deadline_hasher.clone(),
                deadline_clock.clone(),
            )
            .context("cannot open PostgreSQL deadline dispatcher")?;
            let worker = infrastructure::PostgresDeadlineWorkerStore::open(
                database,
                deadline_hasher.clone(),
                deadline_clock.clone(),
            )
            .context("cannot open PostgreSQL deadline worker")?;
            let dashboard = application::dashboard::DashboardService::new(
                Arc::new(
                    infrastructure::PostgresDashboardStore::open(
                        database,
                        deadline_hasher.clone(),
                        deadline_clock.clone(),
                    )
                    .context("cannot open PostgreSQL dashboard store")?,
                ),
                identity.clone(),
            );
            let agenda = application::agenda::AgendaService::new(
                Arc::new(
                    infrastructure::PostgresAgendaStore::open(
                        database,
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
                        database,
                        deadline_hasher.clone(),
                        deadline_clock.clone(),
                    )
                    .context("cannot open PostgreSQL deadline store")?,
                ),
                identity.clone(),
                deadline_hasher,
                deadline_clock,
            );
            let document_content = application::document_content::DocumentContentService::new(
                repository.clone(),
                identity.clone(),
                processor.clone(),
                Arc::new(SystemClock::new()),
                repository.clone(),
            );
            let document_integrity = application::document_integrity::DocumentIntegrityService::new(
                repository.clone(),
                identity.clone(),
                Arc::new(SystemClock::new()),
            );
            let workflow = CaseDocumentService::new(
                repository,
                identity.clone(),
                processor,
                Arc::new(admission),
                Arc::new(SystemClock::new()),
            );
            let alerts = crate::serve_alert_composition::open(
                database,
                identity.clone(),
                email.alerts,
                alert_config,
            )?;
            let resource_activities =
                crate::serve_resource_activities::open(database, identity.clone())?;
            let members = application::members::MemberService::new(
                Arc::new(
                    infrastructure::PostgresMemberStore::open(
                        database,
                        Arc::new(SystemClock::new()),
                    )
                    .context("cannot open PostgreSQL member store")?,
                ),
                identity.clone(),
                Arc::new(SystemClock::new()),
            );
            let resource_deadlines =
                crate::serve_resource_activities::open_deadlines(database, identity.clone())?;
            let audit_events = crate::serve_audit_composition::open(database, identity.clone())?;
            let recovery = crate::serve_password_reset_composition::open(
                database,
                &redis_url,
                email.password_reset,
                budget.clone(),
                stop.clone(),
            )?;
            let (recovery_http, recovery_runtime) = match recovery {
                Some(value) => (Some(value.http), Some(value.runtime)),
                None => (None, None),
            };
            let owner_certificates = owner_certificates::open(database, identity.clone())?;
            let router = web::api_router_with_authentication_budget(
                Arc::new(workflow),
                identity,
                web::CaseWorkflows {
                    owner_certificates,
                    members: Arc::new(members),
                    cases: Arc::new(cases),
                    participants: Arc::new(participants),
                    stages: Arc::new(stages),
                    typed: Arc::new(typed_participants),
                    hearings: Arc::new(hearings),
                    hearing_results: Arc::new(hearing_results),
                    procedural_facts: Arc::new(procedural_facts),
                    procedural_resources: Arc::new(procedural_resources),
                    resource_activities,
                    resource_deadlines,
                    deadlines: Arc::new(deadlines),
                    agenda: Arc::new(agenda),
                    dashboard: Arc::new(dashboard),
                    audit_events,
                    case_reports: reports.workflow,
                    alerts: alerts.workflow,
                    document_content: Arc::new(document_content),
                    document_integrity: Arc::new(document_integrity),
                },
                Arc::new(calendars),
                Arc::new(profiles),
                limits,
                web::AuthenticationHttp {
                    password_reset: recovery_http,
                    certificate_login: identity_components.certificate_login,
                },
                budget,
            )?;
            let server = crate::serve_start::ServerComponents {
                router,
                stop,
                password_reset: recovery_runtime,
            };
            Ok((server, dispatch, worker, alerts.consumers, reports.consumer))
        })?;

    crate::serve_start::run(
        &args.bind,
        server,
        dispatch,
        worker,
        deadline_config,
        alerts,
        reports,
    )
}
