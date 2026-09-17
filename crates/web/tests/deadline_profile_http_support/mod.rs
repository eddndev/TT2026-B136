#![allow(dead_code)]
use application::{deadline_profiles::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod domain_values;
mod values;
pub use values::*;
pub const BASE: &str = "/api/v1/deadline-profiles";
pub const ID: &str = "00000000-0000-0000-0000-000000000000";
pub const CASE: &str = "00000000-0000-0000-0000-000000000003";
pub fn private_base() -> String {
    format!("/api/v1/cases/{CASE}/deadline-profiles")
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x77; 32])
}
pub fn id() -> DeadlineProfileId {
    DeadlineProfileId::from_uuid(Uuid::nil())
}
pub fn collection_json(c: DeadlineProfileCollection) -> Value {
    match c {
        DeadlineProfileCollection::Global => json!({"kind":"global"}),
        DeadlineProfileCollection::ForCase(id) => json!({"kind":"case","case_id":id.to_string()}),
    }
}
pub fn detail(
    c: DeadlineProfileCollection,
    command: &DeadlineProfileCommand,
) -> DeadlineProfileDetail {
    let case = match c {
        DeadlineProfileCollection::Global => None,
        DeadlineProfileCollection::ForCase(id) => Some(id),
    };
    let definition = match &command.change {
        DeadlineProfileChange::Publish { definition }
        | DeadlineProfileChange::Replace { definition, .. } => definition.clone(),
        _ => domain_values::definition(case),
    };
    DeadlineProfileDetail {
        id: command.profile_id,
        revision: command.result_revision().unwrap(),
        definition,
        definition_digest: digest(),
        algorithm: DeadlineProfileAlgorithm::V1,
        status: command.result_status(),
        reason: command.reason().cloned(),
        receipt: DeadlineProfileReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            submission_digest: digest(),
        },
        recorded_at: time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        recorded_by: DeadlineProfileActorSnapshot {
            id: UserId::from_uuid(Uuid::nil()),
            email: "owner@example.test".into(),
        },
    }
}
pub fn published(c: DeadlineProfileCollection) -> DeadlineProfileDetail {
    let case = match c {
        DeadlineProfileCollection::Global => None,
        DeadlineProfileCollection::ForCase(id) => Some(id),
    };
    detail(
        c,
        &DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::from_uuid(Uuid::nil()),
            profile_id: id(),
            change: DeadlineProfileChange::Publish {
                definition: domain_values::definition(case),
            },
        },
    )
}
#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub command: Mutex<Option<DeadlineProfileCommand>>,
    pub response: Mutex<Option<DeadlineProfileDetail>>,
    pub failure: Mutex<Option<&'static str>>,
}
impl Workflow {
    fn record(
        &self,
        token: &str,
        c: DeadlineProfileCollection,
        call: Value,
        write: bool,
    ) -> Result<(), ApplicationError> {
        self.calls
            .lock()
            .unwrap()
            .push(json!([token, collection_json(c), call]));
        if token == "client" || write && token != "owner" {
            return Err(ApplicationError::PermissionDenied);
        }
        match *self.failure.lock().unwrap() {
            Some("case") => Err(ApplicationError::CaseNotFound),
            Some("closed") => Err(ApplicationError::CaseClosed),
            Some("session") => Err(ApplicationError::InvalidSession),
            Some("conflict") => Err(DeadlineProfileError::RevisionConflict.into()),
            Some("mismatch") => Err(DeadlineProfileError::SubmissionMismatch.into()),
            Some("retired") => Err(DeadlineProfileError::Retired.into()),
            Some("not_found") => Err(DeadlineProfileError::NotFound.into()),
            _ => Ok(()),
        }
    }
}
impl DeadlineProfileWorkflow for Workflow {
    fn list(
        &self,
        t: &str,
        c: DeadlineProfileCollection,
        q: DeadlineProfileQuery,
    ) -> Result<DeadlineProfilePage, ApplicationError> {
        self.record(
            t,
            c,
            json!([
                "list",
                q.limit(),
                q.after_id().map(|id| id.to_string()),
                q.status().status().map(|s| s.as_str())
            ]),
            false,
        )?;
        let row = self
            .response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| published(c));
        Ok(DeadlineProfilePage {
            profiles: vec![DeadlineProfileOverview::from(&row)],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        t: &str,
        c: DeadlineProfileCollection,
        id: DeadlineProfileId,
        r: Option<DeadlineProfileRevision>,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        self.record(
            t,
            c,
            json!(["get", id.to_string(), r.map(|v| v.get())]),
            false,
        )?;
        Ok(self
            .response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| published(c)))
    }
    fn history(
        &self,
        t: &str,
        c: DeadlineProfileCollection,
        id: DeadlineProfileId,
        q: DeadlineProfileHistoryQuery,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError> {
        self.record(
            t,
            c,
            json!([
                "history",
                id.to_string(),
                q.limit(),
                q.before_revision().map(|v| v.get())
            ]),
            false,
        )?;
        let row = self
            .response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| published(c));
        Ok(DeadlineProfileHistoryPage {
            revisions: vec![DeadlineProfileHistoryEntry::from(&row)],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn prepare(
        &self,
        t: &str,
        c: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
    ) -> Result<DeadlineProfileDraft, ApplicationError> {
        self.record(
            t,
            c,
            json!([
                "prepare",
                command.profile_id.to_string(),
                command.action().as_str()
            ]),
            true,
        )?;
        *self.command.lock().unwrap() = Some(command.clone());
        let v = detail(c, &command);
        Ok(DeadlineProfileDraft {
            collection: c,
            actor: v.recorded_by.id,
            result_revision: v.revision,
            initial_scope: v.definition.scope().clone(),
            command,
            definition: v.definition,
            definition_digest: digest(),
            algorithm: v.algorithm,
            submission_digest: digest(),
        })
    }
    fn submit(
        &self,
        t: &str,
        c: DeadlineProfileCollection,
        command: DeadlineProfileCommand,
        expected: Sha256Digest,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        self.record(
            t,
            c,
            json!([
                "submit",
                command.profile_id.to_string(),
                command.action().as_str(),
                expected.to_hex()
            ]),
            true,
        )?;
        *self.command.lock().unwrap() = Some(command.clone());
        Ok(self
            .response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| detail(c, &command)))
    }
}
pub async fn request(
    w: Arc<Workflow>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<String>,
    types: &[&str],
) -> (u16, Value) {
    let mut r = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        r = r.header("authorization", format!("Bearer {t}"));
    }
    for t in types {
        r = r.header("content-type", *t);
    }
    let response = web::deadline_profile_router(w)
        .oneshot(
            r.body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn prepare_json(w: Arc<Workflow>, value: Value) -> (u16, Value) {
    request(
        w,
        "POST",
        &format!("{BASE}/prepare"),
        Some("owner"),
        Some(value.to_string()),
        &["application/json"],
    )
    .await
}
pub fn case() -> CaseId {
    CaseId::from_uuid(CASE.parse().unwrap())
}
