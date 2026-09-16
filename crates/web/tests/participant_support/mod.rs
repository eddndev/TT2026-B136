#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use application::participants::*;
use application::ApplicationError;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use time::{OffsetDateTime, UtcOffset};
use tower::ServiceExt;
use uuid::Uuid;

pub const CASE: &str = "00000000-0000-0000-0000-000000000001";
pub const ID: &str = "00000000-0000-0000-0000-000000000002";
pub const ACTOR: &str = "00000000-0000-0000-0000-000000000003";

pub fn base() -> String {
    format!("/api/v1/cases/{CASE}/participants")
}
pub fn item() -> String {
    format!("{}/{ID}", base())
}
pub fn values() -> ParticipantValues {
    ParticipantValues::new("Ana", "Witness", None, None, DirectoryStatus::Active).unwrap()
}
pub fn snapshot() -> ParticipantSnapshot {
    ParticipantSnapshot {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        id: ParticipantId::from_uuid(Uuid::from_u128(2)),
        revision: ParticipantRevision::new(3).unwrap(),
        values: values(),
        values_digest: Sha256Digest::from_bytes(&[0x1a; 32]).unwrap(),
        changed_at: OffsetDateTime::from_unix_timestamp(1_700_000_000)
            .unwrap()
            .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()),
        changed_by: ParticipantActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(3)),
            email: "historical@example.com".into(),
        },
    }
}
pub fn replacement(expected: u32) -> Value {
    json!({"expected_revision":expected,"display_name":" Ana ",
        "procedural_role":" Witness ","organization":" ",
        "legal_status":null,"directory_status":"active"})
}

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub filters: Mutex<Vec<Value>>,
}

impl Workflow {
    fn record(&self, token: &str, call: Value) -> Result<ParticipantSnapshot, ApplicationError> {
        self.calls.lock().unwrap().push(call);
        match token {
            "expired" => Err(ApplicationError::InvalidSession),
            "forbidden" => Err(ApplicationError::PermissionDenied),
            "hidden-case" => Err(ApplicationError::CaseNotFound),
            "hidden" => Err(ApplicationError::ParticipantNotFound),
            "conflict" => Err(ApplicationError::ParticipantRevisionConflict),
            "exhausted" => Err(ApplicationError::ParticipantRevisionExhausted),
            "corrupt" => Err(ApplicationError::StoredParticipantInconsistent(
                "secret row".into(),
            )),
            "failed" => Err(ApplicationError::Port("secret DSN".into())),
            _ => Ok(snapshot()),
        }
    }
}

impl ParticipantWorkflow for Workflow {
    fn create(
        &self,
        token: &str,
        case: CaseId,
        name: &str,
        role: &str,
        organization: Option<&str>,
        legal_status: Option<&str>,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        self.record(
            token,
            json!([
                "create",
                token,
                case,
                name,
                role,
                organization,
                legal_status
            ]),
        )
    }
    fn replace(
        &self,
        token: &str,
        case: CaseId,
        id: ParticipantId,
        expected: ParticipantRevision,
        values: ParticipantValues,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        self.record(
            token,
            json!(["replace", token, case, id, expected.get(), values]),
        )
    }
    fn change_status(
        &self,
        token: &str,
        case: CaseId,
        id: ParticipantId,
        expected: ParticipantRevision,
        status: DirectoryStatus,
    ) -> Result<ParticipantDetail, ApplicationError> {
        self.record(
            token,
            json!(["status", token, case, id, expected.get(), status]),
        )
        .map(Into::into)
    }
    fn list(
        &self,
        token: &str,
        case: CaseId,
        query: ParticipantQuery,
    ) -> Result<ParticipantPage, ApplicationError> {
        self.filters.lock().unwrap().push(json!([
            query.kind().map(|v| v.as_str()),
            match query.profile() {
                ParticipantProfileFilter::All => "all",
                ParticipantProfileFilter::Manual => "manual",
                ParticipantProfileFilter::Typed => "typed",
            }
        ]));
        let row = self.record(
            token,
            json!([
                "list",
                token,
                case,
                query.limit(),
                query.after_id(),
                query.name(),
                query.procedural_role(),
                query.status().directory_status()
            ]),
        )?;
        Ok(ParticipantPage {
            next_after_id: Some(row.id),
            participants: vec![row.into()],
            has_more: true,
        })
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        id: ParticipantId,
    ) -> Result<ParticipantDetail, ApplicationError> {
        self.record(token, json!(["get", token, case, id]))
            .map(Into::into)
    }
    fn get_revision(
        &self,
        token: &str,
        case: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
    ) -> Result<ParticipantDetail, ApplicationError> {
        self.record(
            token,
            json!(["get_revision", token, case, id, revision.get()]),
        )
        .map(Into::into)
    }
    fn history(
        &self,
        token: &str,
        case: CaseId,
        id: ParticipantId,
        query: ParticipantHistoryQuery,
    ) -> Result<ParticipantHistoryPage, ApplicationError> {
        let row = self.record(
            token,
            json!([
                "history",
                token,
                case,
                id,
                query.limit(),
                query.before_revision().map(|revision| revision.get())
            ]),
        )?;
        Ok(ParticipantHistoryPage {
            next_before_revision: Some(row.revision),
            revisions: vec![row.into()],
            has_more: true,
        })
    }
}

pub async fn request(
    workflow: &Arc<Workflow>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: impl Into<Body>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::participant_router(workflow.clone())
        .oneshot(request.body(body.into()).unwrap())
        .await
        .unwrap()
}

pub async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 128 * 1024).await.unwrap()).unwrap()
}
