//! Full API composition over a single admission and blocking-work runtime.

use std::sync::Arc;

use application::{documents::CaseDocumentWorkflow, identity::IdentityWorkflow, ApplicationError};
use axum::{routing::get, Router};

use crate::{
    agenda, alerts, audit_events, case_administration, case_reports, case_stages, cases, dashboard,
    deadline_profiles, deadlines, document_content, document_integrity, health, hearing_results,
    hearings, judicial_calendars, members, owner_certificates, participants,
    password_reset::{self, PasswordResetHttp},
    procedural_facts, procedural_resources, resource_activities, resource_deadlines, routes,
    runtime::{protect, HttpRuntime},
    typed_participants, CaseWorkflows, HttpLimits, HttpWorkBudget,
};

/// Builds all API routes with password reset unavailable and one shared budget.
pub fn api_router(
    documents: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    workflows: CaseWorkflows,
    calendars: Arc<dyn application::judicial_calendars::JudicialCalendarWorkflow>,
    profiles: Arc<dyn application::deadline_profiles::DeadlineProfileWorkflow>,
    limits: HttpLimits,
) -> Router {
    api_router_with_password_reset(
        documents, identity, workflows, calendars, profiles, limits, None,
    )
}

/// Builds all API routes with optional reset components and one shared budget.
///
/// Absent components keep the reset endpoints available with a neutral
/// unavailable response. Health remains outside request admission.
pub fn api_router_with_password_reset(
    documents: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    workflows: CaseWorkflows,
    calendars: Arc<dyn application::judicial_calendars::JudicialCalendarWorkflow>,
    profiles: Arc<dyn application::deadline_profiles::DeadlineProfileWorkflow>,
    limits: HttpLimits,
    password_reset: Option<PasswordResetHttp>,
) -> Router {
    let budget = HttpWorkBudget::new(limits.max_blocking_operations);
    api_router_with_password_reset_budget(
        documents,
        identity,
        workflows,
        calendars,
        profiles,
        limits,
        password_reset,
        budget,
    )
    .expect("a newly created work budget matches its declared capacity")
}

/// Builds the API over the same blocking-work budget used by external consumers.
/// The budget's fixed capacity must match the declared blocking-operation limit.
// Preserve the existing composition inputs while adding the shared budget handle.
#[allow(clippy::too_many_arguments)]
pub fn api_router_with_password_reset_budget(
    documents: Arc<dyn CaseDocumentWorkflow>,
    identity: Arc<dyn IdentityWorkflow>,
    workflows: CaseWorkflows,
    calendars: Arc<dyn application::judicial_calendars::JudicialCalendarWorkflow>,
    profiles: Arc<dyn application::deadline_profiles::DeadlineProfileWorkflow>,
    limits: HttpLimits,
    password_reset: Option<PasswordResetHttp>,
    budget: HttpWorkBudget,
) -> Result<Router, ApplicationError> {
    let runtime = HttpRuntime::with_budget(limits, budget)?;
    let routes = routes::router(documents, identity, runtime.clone())
        .merge(owner_certificates::router(
            workflows.owner_certificates,
            runtime.clone(),
        ))
        .merge(members::router(workflows.members, runtime.clone()))
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
        .merge(procedural_resources::router(
            workflows.procedural_resources,
            runtime.clone(),
        ))
        .merge(resource_activities::router(
            workflows.resource_activities,
            runtime.clone(),
        ))
        .merge(resource_deadlines::router(
            workflows.resource_deadlines,
            runtime.clone(),
        ))
        .merge(deadlines::router(workflows.deadlines, runtime.clone()))
        .merge(agenda::router(workflows.agenda, runtime.clone()))
        .merge(dashboard::router(workflows.dashboard, runtime.clone()))
        .merge(audit_events::router(
            workflows.audit_events,
            runtime.clone(),
        ))
        .merge(case_reports::router(
            workflows.case_reports,
            runtime.clone(),
        ))
        .merge(alerts::router(workflows.alerts, runtime.clone()))
        .merge(document_content::router(
            workflows.document_content,
            runtime.clone(),
        ))
        .merge(document_integrity::router(
            workflows.document_integrity,
            runtime.clone(),
        ))
        .merge(judicial_calendars::router(calendars, runtime.clone()))
        .merge(deadline_profiles::router(profiles, runtime.clone()))
        .merge(password_reset::router(password_reset, runtime.clone()));
    Ok(protect(routes, runtime).route("/healthz", get(health)))
}
