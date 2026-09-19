//! Inbound HTTP adapter.
//!
//! Routes adapt HTTP to injected identity, document, case and participant workflows.
//! Cryptography, persistence, and timestamps remain behind application ports,
//! so this adapter does not depend on an external timestamp provider.

use std::sync::Arc;

use application::case_stages::CaseStageWorkflow;
use application::cases::CaseWorkflow;
use application::documents::CaseDocumentWorkflow;
use application::identity::IdentityWorkflow;
use application::participants::ParticipantWorkflow;
use axum::{routing::get, Router};

mod agenda;
mod alerts;
mod case_administration;
mod case_stages;
mod cases;
mod deadline_profiles;
mod deadlines;
mod dto;
mod error;
mod hearing_results;
mod hearings;
mod judicial_calendars;
mod participants;
mod procedural_facts;
mod request;
mod routes;
mod runtime;
mod typed_participants;
pub use runtime::HttpLimits;
use runtime::{protect, HttpRuntime};

/// Builds the inbound HTTP router.
pub fn router() -> Router {
    Router::new().route("/healthz", get(health))
}

/// Builds the versioned application API over an injected workflow.
pub fn application_router(
    workflow: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(routes::router(workflow, identity, runtime.clone()), runtime)
        .route("/healthz", get(health))
}

/// Builds case routes whose workflow authenticates and authorizes each request.
pub fn case_router(workflow: Arc<dyn CaseWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(cases::router(workflow, runtime.clone()), runtime)
}

/// Builds staff case administration routes with workflow authorization.
pub fn case_administration_router(workflow: Arc<dyn CaseWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        case_administration::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Builds stage routes whose workflow validates support and case authorization.
pub fn case_stage_router(workflow: Arc<dyn CaseStageWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(case_stages::router(workflow, runtime.clone()), runtime)
}

/// Builds participant routes with authentication delegated to the workflow.
pub fn participant_router(workflow: Arc<dyn ParticipantWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(participants::router(workflow, runtime.clone()), runtime)
}

/// Builds reviewed participant and subject routes over an authorized workflow.
pub fn typed_participant_router(
    workflow: Arc<dyn application::typed_participants::TypedParticipantWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        typed_participants::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Builds the combined authorized hearing and operational deadline agenda.
pub fn agenda_router(workflow: Arc<dyn application::agenda::AgendaWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(agenda::router(workflow, runtime.clone()), runtime)
}

/// Builds personal alert preferences and inbox routes over an authorized workflow.
pub fn alert_router(workflow: Arc<dyn application::alerts::AlertWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(alerts::router(workflow, runtime.clone()), runtime)
}

/// Builds authorized hearing and global agenda routes.
pub fn hearing_router(workflow: Arc<dyn application::hearings::HearingWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(hearings::router(workflow, runtime.clone()), runtime)
}

/// Builds authorized routes for declared hearing sessions and their exact history.
pub fn hearing_result_router(
    workflow: Arc<dyn application::hearing_results::HearingResultWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(hearing_results::router(workflow, runtime.clone()), runtime)
}

/// Builds authorized declarations and exact history for both procedural fact families.
pub fn procedural_fact_router(
    workflow: Arc<dyn application::procedural_facts::ProceduralFactWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(procedural_facts::router(workflow, runtime.clone()), runtime)
}

/// Builds global staff calendar routes with application authorization.
pub fn judicial_calendar_router(
    workflow: Arc<dyn application::judicial_calendars::JudicialCalendarWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        judicial_calendars::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Related case workflows injected together into the shared HTTP runtime.
pub struct CaseWorkflows {
    pub cases: Arc<dyn CaseWorkflow>,
    pub participants: Arc<dyn ParticipantWorkflow>,
    pub stages: Arc<dyn CaseStageWorkflow>,
    pub typed: Arc<dyn application::typed_participants::TypedParticipantWorkflow>,
    pub hearings: Arc<dyn application::hearings::HearingWorkflow>,
    pub hearing_results: Arc<dyn application::hearing_results::HearingResultWorkflow>,
    pub procedural_facts: Arc<dyn application::procedural_facts::ProceduralFactWorkflow>,
    pub deadlines: Arc<dyn application::deadlines::DeadlineWorkflow>,
    pub agenda: Arc<dyn application::agenda::AgendaWorkflow>,
}

/// Builds the explicit global and case profile collections over an authorized workflow.
pub fn deadline_profile_router(
    workflow: Arc<dyn application::deadline_profiles::DeadlineProfileWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        deadline_profiles::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Builds case deadline routes with exact historical results and workflow authorization.
pub fn deadline_router(workflow: Arc<dyn application::deadlines::DeadlineWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(deadlines::router(workflow, runtime.clone()), runtime)
}

/// Builds all API routes with one shared admission and blocking-work budget.
pub fn api_router(
    documents: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    workflows: CaseWorkflows,
    calendars: Arc<dyn application::judicial_calendars::JudicialCalendarWorkflow>,
    profiles: Arc<dyn application::deadline_profiles::DeadlineProfileWorkflow>,
    limits: HttpLimits,
) -> Router {
    let runtime = HttpRuntime::new(limits);
    let routes = routes::router(documents, identity, runtime.clone())
        .merge(cases::router(workflows.cases.clone(), runtime.clone()))
        .merge(case_administration::router(
            workflows.cases,
            runtime.clone(),
        ))
        .merge(participants::router(
            workflows.participants,
            runtime.clone(),
        ))
        .merge(case_stages::router(workflows.stages, runtime.clone()))
        .merge(typed_participants::router(workflows.typed, runtime.clone()))
        .merge(hearings::router(workflows.hearings, runtime.clone()))
        .merge(hearing_results::router(
            workflows.hearing_results,
            runtime.clone(),
        ))
        .merge(procedural_facts::router(
            workflows.procedural_facts,
            runtime.clone(),
        ))
        .merge(deadlines::router(workflows.deadlines, runtime.clone()))
        .merge(agenda::router(workflows.agenda, runtime.clone()))
        .merge(judicial_calendars::router(calendars, runtime.clone()))
        .merge(deadline_profiles::router(profiles, runtime.clone()));
    protect(routes, runtime).route("/healthz", get(health))
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::router;
    use axum::{body::to_bytes, body::Body, http::Request, http::StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_reports_ready() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }
}
