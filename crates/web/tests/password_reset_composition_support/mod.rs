#[path = "../resource_activity_http_support/mod.rs"]
mod activities;
#[path = "../alert_http_support/mod.rs"]
mod alerts;
#[allow(dead_code)]
#[path = "../auth_support/mod.rs"]
mod auth;
#[allow(dead_code)]
#[path = "../judicial_calendar_support/mod.rs"]
mod calendars;
#[allow(dead_code)]
#[path = "../case_api_support/mod.rs"]
mod cases;
mod dashboard;
#[path = "../deadline_http_support/mod.rs"]
mod deadlines;
#[path = "../procedural_fact_http_support/mod.rs"]
mod facts;
#[allow(dead_code)]
#[path = "../hearing_result_support/mod.rs"]
mod hearing_results;
#[path = "../document_support/identity.rs"]
mod identity;
#[allow(dead_code)]
#[path = "../member_http_support/mod.rs"]
mod members;
#[path = "../participant_support/mod.rs"]
mod participants;
#[allow(unused_imports)]
#[path = "../deadline_profile_http_support/mod.rs"]
mod profiles;
#[path = "../case_report_http_support/mod.rs"]
mod reports;
#[path = "../procedural_resource_http_support/mod.rs"]
mod resources;
#[path = "../case_stage_support/mod.rs"]
mod stages;
mod unused;
mod unused_owner;
mod unused_typed;

use crate::password_reset_http_support::{Ports, RequestAdmission, Requests};
use application::identity::owner_certificates::OwnerCertificateService;
use application::identity::password_reset::{
    PasswordResetPorts, PasswordResetService, ResetPolicy,
};
use application::ApplicationError;
use axum::Router;
pub use dashboard::{get, Dashboard};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use web::password_reset::PasswordResetHttp;
use web::{
    api_router, api_router_with_password_reset, api_router_with_password_reset_budget,
    CaseWorkflows, HttpLimits, HttpWorkBudget,
};

pub struct Harness {
    pub router: Router,
    pub dashboard: Arc<Dashboard>,
    pub requests: Arc<Requests>,
    pub ports: Arc<Ports>,
}

impl Harness {
    pub fn new(max_requests: usize, legacy: bool) -> Self {
        Self::build(max_requests, 1, legacy, None, None).unwrap()
    }

    pub fn with_budget(
        max_requests: usize,
        max_blocking: usize,
        budget: HttpWorkBudget,
    ) -> Result<Self, ApplicationError> {
        Self::build(max_requests, max_blocking, false, Some(budget), None)
    }

    #[allow(dead_code)]
    pub fn with_owner_certificate(
        max_requests: usize,
        budget: HttpWorkBudget,
        owner: Arc<OwnerCertificateService>,
    ) -> Result<Self, ApplicationError> {
        Self::build(
            max_requests,
            budget.capacity(),
            false,
            Some(budget),
            Some(owner),
        )
    }

    fn build(
        max_requests: usize,
        max_blocking: usize,
        legacy: bool,
        budget: Option<HttpWorkBudget>,
        owner: Option<Arc<OwnerCertificateService>>,
    ) -> Result<Self, ApplicationError> {
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
        let limits = HttpLimits {
            max_requests: NonZeroUsize::new(max_requests).unwrap(),
            max_blocking_operations: NonZeroUsize::new(max_blocking).unwrap(),
        };
        let documents = Arc::new(auth::UnusedDocuments);
        let identity = Arc::new(identity::StubIdentity);
        let workflows = workflows(dashboard.clone(), owner);
        let calendars = Arc::new(calendars::Workflow::default());
        let profiles = Arc::new(profiles::Workflow::default());
        let router = if legacy {
            api_router(documents, identity, workflows, calendars, profiles, limits)
        } else if let Some(budget) = budget {
            api_router_with_password_reset_budget(
                documents,
                identity,
                workflows,
                calendars,
                profiles,
                limits,
                Some(PasswordResetHttp {
                    requests: requests.clone(),
                    completion,
                }),
                budget,
            )?
        } else {
            api_router_with_password_reset(
                documents,
                identity,
                workflows,
                calendars,
                profiles,
                limits,
                Some(PasswordResetHttp {
                    requests: requests.clone(),
                    completion,
                }),
            )
        };
        Ok(Self {
            router,
            dashboard,
            requests,
            ports,
        })
    }
}

fn workflows(
    dashboard: Arc<Dashboard>,
    owner: Option<Arc<OwnerCertificateService>>,
) -> CaseWorkflows {
    let unused = Arc::new(unused::Unused);
    CaseWorkflows {
        owner_certificates: owner.unwrap_or_else(unused_owner::service),
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

// The binary acceptance supplies its durable identity and recovery adapters here.
#[allow(dead_code)]
pub fn router_with_identity_reset(
    identity: Arc<dyn application::identity::IdentityWorkflow>,
    reset: PasswordResetHttp,
    limits: HttpLimits,
    budget: HttpWorkBudget,
) -> Router {
    api_router_with_password_reset_budget(
        Arc::new(auth::UnusedDocuments),
        identity,
        workflows(Arc::new(Dashboard::default()), None),
        Arc::new(calendars::Workflow::default()),
        Arc::new(profiles::Workflow::default()),
        limits,
        Some(reset),
        budget,
    )
    .unwrap()
}
