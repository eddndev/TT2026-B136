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
mod api;
mod audit_events;
mod case_administration;
mod case_stages;
mod cases;
mod dashboard;
mod deadline_profiles;
mod deadlines;
mod document_content;
mod document_integrity;
mod dto;
mod error;
mod hearing_results;
mod hearings;
mod judicial_calendars;
mod members;
mod owner_certificates;
mod owner_login;
mod participants;
pub mod password_reset;
mod procedural_facts;
mod procedural_resources;
mod request;
mod resource_activities;
mod resource_deadlines;
mod resource_hearings;
mod routes;
mod runtime;
mod typed_participants;
pub use api::{
    api_router, api_router_with_authentication_budget, api_router_with_password_reset,
    api_router_with_password_reset_budget, AuthenticationHttp,
};
use runtime::{protect, HttpRuntime};
pub use runtime::{HttpLimits, HttpWorkBudget, HttpWorkPermit};

/// Builds standalone reset routes using one admission and blocking-work budget.
pub fn password_reset_router(
    components: Option<password_reset::PasswordResetHttp>,
    limits: HttpLimits,
) -> Router {
    let runtime = HttpRuntime::new(limits);
    protect(password_reset::router(components, runtime.clone()), runtime)
}

/// Builds self-Owner certificate binding routes with bounded blocking work.
pub fn owner_certificate_router(
    service: Arc<application::identity::owner_certificates::OwnerCertificateService>,
    limits: HttpLimits,
) -> Router {
    let runtime = HttpRuntime::new(limits);
    protect(
        owner_certificates::router(service, runtime.clone()),
        runtime,
    )
}

/// Builds bounded public certificate first-factor routes that require later MFA.
pub fn owner_login_router(
    workflow: Arc<dyn application::identity::certificate_login::OwnerLoginWorkflow>,
    limits: HttpLimits,
) -> Router {
    let runtime = HttpRuntime::new(limits);
    protect(
        owner_login::router(Some(workflow), runtime.clone()),
        runtime,
    )
}

/// Builds the inbound HTTP router.
pub fn router() -> Router {
    Router::new().route("/healthz", get(health))
}

/// Builds Owner directory and account access routes.
pub fn member_router(workflow: Arc<dyn application::members::MemberWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(members::router(workflow, runtime.clone()), runtime)
}

/// Builds exact document content routes over an authenticated workflow.
pub fn document_content_router(
    workflow: Arc<dyn application::document_content::DocumentContentWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(document_content::router(workflow, runtime.clone()), runtime)
}

/// Builds the Owner-only document integrity incident inbox.
pub fn document_integrity_router(
    workflow: Arc<dyn application::document_integrity::DocumentIntegrityWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        document_integrity::router(workflow, runtime.clone()),
        runtime,
    )
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

/// Builds bounded Owner consultation of existing recorded audit activity.
pub fn audit_events_router(
    workflow: Arc<dyn application::audit_query::AuditEventWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(audit_events::router(workflow, runtime.clone()), runtime)
}

/// Builds the authorized operational dashboard.
pub fn dashboard_router(workflow: Arc<dyn application::dashboard::DashboardWorkflow>) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(dashboard::router(workflow, runtime.clone()), runtime)
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

/// Builds authorized resources, declared acts and exact immutable history.
pub fn procedural_resource_router(
    workflow: Arc<dyn application::procedural_resources::ProceduralResourceWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        procedural_resources::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Builds exact resource association routes over an authorized workflow.
pub fn resource_activity_router(
    workflow: Arc<dyn application::resource_activities::ResourceActivityWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        resource_activities::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Builds atomic contextual deadline creation over an authorized workflow.
pub fn resource_deadline_router(
    workflow: Arc<dyn application::resource_deadlines::ResourceDeadlineWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        resource_deadlines::router(workflow, runtime.clone()),
        runtime,
    )
}

/// Builds declared resource hearing submission and authorized historical reads.
pub fn resource_hearing_router(
    workflow: Arc<dyn application::resource_hearings::ResourceHearingWorkflow>,
    reads: Arc<dyn application::resource_hearings::ResourceHearingReadWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(
        resource_hearings::router(workflow, reads, runtime.clone()),
        runtime,
    )
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

/// Identity evidence and case workflows injected into the shared HTTP runtime.
pub struct CaseWorkflows {
    pub owner_certificates: Arc<application::identity::owner_certificates::OwnerCertificateService>,
    pub members: Arc<dyn application::members::MemberWorkflow>,
    pub cases: Arc<dyn CaseWorkflow>,
    pub participants: Arc<dyn ParticipantWorkflow>,
    pub stages: Arc<dyn CaseStageWorkflow>,
    pub typed: Arc<dyn application::typed_participants::TypedParticipantWorkflow>,
    pub hearings: Arc<dyn application::hearings::HearingWorkflow>,
    pub hearing_results: Arc<dyn application::hearing_results::HearingResultWorkflow>,
    pub procedural_facts: Arc<dyn application::procedural_facts::ProceduralFactWorkflow>,
    pub procedural_resources:
        Arc<dyn application::procedural_resources::ProceduralResourceWorkflow>,
    pub resource_activities: Arc<dyn application::resource_activities::ResourceActivityWorkflow>,
    pub resource_deadlines: Arc<dyn application::resource_deadlines::ResourceDeadlineWorkflow>,
    pub resource_hearings: Arc<dyn application::resource_hearings::ResourceHearingWorkflow>,
    pub resource_hearing_reads:
        Arc<dyn application::resource_hearings::ResourceHearingReadWorkflow>,
    pub deadlines: Arc<dyn application::deadlines::DeadlineWorkflow>,
    pub agenda: Arc<dyn application::agenda::AgendaWorkflow>,
    pub dashboard: Arc<dyn application::dashboard::DashboardWorkflow>,
    pub audit_events: Arc<dyn application::audit_query::AuditEventWorkflow>,
    pub case_reports: Arc<dyn application::case_reports::CaseReportWorkflow>,
    pub alerts: Arc<dyn application::alerts::AlertWorkflow>,
    pub document_content: Arc<dyn application::document_content::DocumentContentWorkflow>,
    pub document_integrity: Arc<dyn application::document_integrity::DocumentIntegrityWorkflow>,
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

mod case_reports;
/// Builds authenticated durable report request and download routes.
pub fn case_reports_router(
    workflow: Arc<dyn application::case_reports::CaseReportWorkflow>,
) -> Router {
    let runtime = HttpRuntime::new(HttpLimits::default());
    protect(case_reports::router(workflow, runtime.clone()), runtime)
}
