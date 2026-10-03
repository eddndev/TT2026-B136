#[path = "../resource_activity_http_support/mod.rs"]
mod activities;
#[path = "../alert_http_support/mod.rs"]
mod alerts;
#[path = "../auth_support/mod.rs"]
mod auth;
#[path = "../judicial_calendar_support/mod.rs"]
mod calendars;
#[path = "../case_api_support/mod.rs"]
mod cases;
#[path = "../password_reset_composition_support/dashboard.rs"]
mod dashboard;
#[path = "../deadline_http_support/mod.rs"]
mod deadlines;
#[path = "../procedural_fact_http_support/mod.rs"]
mod facts;
#[path = "../hearing_result_support/mod.rs"]
mod hearing_results;
#[path = "../document_support/identity.rs"]
mod identity;
#[path = "../member_http_support/mod.rs"]
mod members;
#[path = "../participant_support/mod.rs"]
mod participants;
#[path = "../deadline_profile_http_support/mod.rs"]
mod profiles;
#[path = "../case_report_http_support/mod.rs"]
mod reports;
#[path = "../procedural_resource_http_support/mod.rs"]
mod resources;
#[path = "../case_stage_support/mod.rs"]
mod stages;
#[path = "../password_reset_composition_support/unused.rs"]
mod unused;
#[path = "../password_reset_composition_support/unused_owner.rs"]
mod unused_owner;
#[path = "../password_reset_composition_support/unused_typed.rs"]
mod unused_typed;

use super::support::limits;
use crate::password_reset_http_support::{Ports, RequestAdmission, Requests};
use application::{
    identity::{
        certificate_login::OwnerLoginWorkflow,
        password_reset::{PasswordResetPorts, PasswordResetService, ResetPolicy},
    },
    ApplicationError,
};
use axum::Router;
pub use dashboard::{get, Dashboard};
use std::sync::{Arc, Mutex};
use web::password_reset::PasswordResetHttp;
use web::{
    api_router_with_authentication_budget, AuthenticationHttp, CaseWorkflows, HttpWorkBudget,
};

pub struct Harness {
    pub router: Router,
    pub dashboard: Arc<Dashboard>,
    pub requests: Arc<Requests>,
    pub ports: Arc<Ports>,
}

pub fn composed(
    max_requests: usize,
    max_blocking: usize,
    budget: HttpWorkBudget,
    certificate_login: Option<Arc<dyn OwnerLoginWorkflow>>,
) -> Result<Harness, ApplicationError> {
    let dashboard = Arc::new(Dashboard::default());
    let requests = Arc::new(Requests {
        admission: RequestAdmission::Accepted,
        calls: Mutex::new(0),
        retained: Mutex::new(Vec::new()),
    });
    let ports = Arc::new(Ports::default());
    let completion = Arc::new(PasswordResetService::new(
        PasswordResetPorts {
            repository: ports.clone(),
            delivery: ports.clone(),
            limiter: ports.clone(),
            tokens: ports.clone(),
            digests: ports.clone(),
            passwords: ports.clone(),
        },
        ResetPolicy::new(60, 2).unwrap(),
    ));
    let router = api_router_with_authentication_budget(
        Arc::new(auth::UnusedDocuments),
        Arc::new(identity::StubIdentity),
        workflows(dashboard.clone()),
        Arc::new(calendars::Workflow::default()),
        Arc::new(profiles::Workflow::default()),
        limits(max_requests, max_blocking),
        AuthenticationHttp {
            password_reset: Some(PasswordResetHttp {
                requests: requests.clone(),
                completion,
            }),
            certificate_login,
        },
        budget,
    )?;
    Ok(Harness {
        router,
        dashboard,
        requests,
        ports,
    })
}

fn workflows(dashboard: Arc<Dashboard>) -> CaseWorkflows {
    let unused = Arc::new(unused::Unused);
    CaseWorkflows {
        owner_certificates: unused_owner::service(),
        members: members::Workflow::new(),
        cases: Arc::new(cases::Workflow::default()),
        participants: Arc::new(participants::Workflow::default()),
        stages: Arc::new(stages::Workflow::default()),
        typed: unused.clone(),
        hearings: unused.clone(),
        hearing_results: Arc::new(hearing_results::Workflow::default()),
        procedural_facts: Arc::new(facts::Workflow::default()),
        procedural_resources: Arc::new(resources::Workflow::default()),
        resource_activities: Arc::new(activities::Workflow::default()),
        resource_deadlines: unused.clone(),
        deadlines: Arc::new(deadlines::Workflow::default()),
        agenda: unused.clone(),
        dashboard,
        audit_events: unused.clone(),
        case_reports: Arc::new(reports::Workflow {
            calls: Mutex::new(Vec::new()),
            ready: false,
            failure: None,
        }),
        alerts: Arc::new(alerts::Workflow {
            calls: 0.into(),
            denied: true,
        }),
        document_content: unused.clone(),
        document_integrity: unused,
    }
}
