#![allow(dead_code)]

mod fixtures;
pub use fixtures::*;

use application::cases::*;
use application::ApplicationError;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, identity::UserId};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
}

impl Workflow {
    fn record(
        &self,
        token: &str,
        call: Value,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.calls.lock().unwrap().push(call);
        match token {
            "expired" => Err(ApplicationError::InvalidSession),
            "forbidden" => Err(ApplicationError::PermissionDenied),
            "hidden" => Err(ApplicationError::CaseNotFound),
            "conflict" => Err(ApplicationError::CaseRevisionConflict),
            "exhausted" => Err(ApplicationError::CaseRevisionExhausted),
            "closed" => Err(ApplicationError::CaseClosed),
            "duplicate" => Err(ApplicationError::CaseIdentifierConflict),
            "profile-required" => Err(ApplicationError::CaseProfileRequired),
            "corrupt" => Err(ApplicationError::StoredCaseAdministrationInconsistent(
                "secret row".into(),
            )),
            "failed" => Err(ApplicationError::Port("secret DSN".into())),
            "baseline" => Ok(CaseAdministrationDetail {
                origin: origin(),
                administration: CurrentCaseAdministration::Unrevised(metadata()),
                initial_stage: None,
            }),
            _ => Ok(detail()),
        }
    }
}

impl CaseWorkflow for Workflow {
    fn create(&self, _: &str, _: &str, _: &str) -> Result<CaseRecord, ApplicationError> {
        panic!("administration routes must not use basic creation")
    }
    fn list(&self, _: &str, _: u32, _: u32) -> Result<Vec<CaseRecord>, ApplicationError> {
        panic!("administration routes must not use the basic index")
    }
    fn get(&self, _: &str, _: CaseId) -> Result<CaseRecord, ApplicationError> {
        panic!("administration commands must not follow up with a basic GET")
    }
    fn assign(&self, _: &str, _: CaseId, _: UserId) -> Result<(), ApplicationError> {
        panic!("administration routes must not change membership")
    }
    fn remove(&self, _: &str, _: CaseId, _: UserId) -> Result<(), ApplicationError> {
        panic!("administration routes must not change membership")
    }
    fn register_penal(
        &self,
        token: &str,
        creation: PenalCaseCreation,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "create",
                token,
                creation.metadata().title(),
                creation.profile().nuc(),
                creation.profile().general_information(),
                creation.profile().offenses(),
                creation.profile().complementary_identifiers()
            ]),
        )
    }
    fn replace_administration(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseRevisionExpectation,
        values: CaseEditableValues,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "replace",
                token,
                id,
                expected.get(),
                values.metadata().title(),
                values.profile().is_some()
            ]),
        )
    }
    fn change_administrative_status(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseRevisionExpectation,
        status: CaseAdministrativeStatus,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.record(
            token,
            json!(["status", token, id, expected.get(), status.as_str()]),
        )
    }
    fn list_administrations(
        &self,
        token: &str,
        query: CaseAdministrationQuery,
    ) -> Result<CaseAdministrationPage, ApplicationError> {
        self.record(
            token,
            json!([
                "list",
                token,
                query.limit(),
                query.after_id(),
                format!("{:?}", query.status()),
                format!("{:?}", query.profile()),
                query.title(),
                query.nuc(),
                query.judicial_case_number()
            ]),
        )?;
        Ok(CaseAdministrationPage {
            cases: vec![CaseAdministrationOverview {
                origin: origin(),
                metadata: metadata(),
                revision: Some(CaseRevision::new(3).unwrap()),
                administrative_status: CaseAdministrativeStatus::Active,
                penal_identifiers: Some(CasePenalIdentifiers {
                    nuc: "NUC-001".into(),
                    judicial_case_number: "CJ-001".into(),
                }),
                initial_stage: Some(InitialCaseStage::Investigation),
            }],
            has_more: true,
            next_after_id: Some(origin().id),
        })
    }
    fn get_administration(
        &self,
        token: &str,
        id: CaseId,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        self.record(token, json!(["get", token, id]))
    }
    fn administration_history(
        &self,
        token: &str,
        id: CaseId,
        query: CaseAdministrationHistoryQuery,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError> {
        self.record(
            token,
            json!([
                "history",
                token,
                id,
                query.limit(),
                query.before_revision().map(|r| r.get())
            ]),
        )?;
        Ok(CaseAdministrationHistoryPage {
            revisions: vec![snapshot()],
            has_more: true,
            next_before_revision: Some(CaseRevision::new(3).unwrap()),
        })
    }
}

pub async fn request(
    workflow: &Arc<Workflow>,
    method: &str,
    path: &str,
    token: Option<&str>,
    input: impl Into<Body>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::case_administration_router(workflow.clone())
        .oneshot(request.body(input.into()).unwrap())
        .await
        .unwrap()
}
pub async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1_048_576).await.unwrap()).unwrap()
}
