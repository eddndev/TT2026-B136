#![allow(dead_code)]
use application::{
    case_stages::*,
    cases::{CaseActorSnapshot, CaseInitialStageRegistration},
    ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    response::Response,
};
use domain::{
    case_administration::{CaseRevision, InitialCaseStage},
    cases::CaseId,
    crypto::Sha256Digest,
    identity::UserId,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use time::{OffsetDateTime, UtcOffset};
use tower::ServiceExt;
use uuid::Uuid;

pub const CASE: &str = "00000000-0000-0000-0000-000000000001";
pub const DOCUMENT: &str = "00000000-0000-0000-0000-000000000002";
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn actor() -> CaseActorSnapshot {
    CaseActorSnapshot {
        id: UserId::from_uuid(Uuid::from_u128(3)),
        email: "owner@example.test".into(),
    }
}
pub fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_700_000_000)
        .unwrap()
        .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap())
}
pub fn initial(id: CaseId) -> CaseStageEntry {
    CaseStageEntry::Initial(CaseInitialStageRegistration {
        case_id: id,
        stage_revision: CaseStageRevision::FIRST,
        administration_revision: CaseRevision::FIRST,
        stage: InitialCaseStage::Investigation,
        administration_digest: Sha256Digest::from_bytes(&[0x11; 32]).unwrap(),
        recorded_at: at(),
        recorded_by: actor(),
    })
}
pub fn detail(id: CaseId, entry: Option<CaseStageEntry>) -> CaseStageDetail {
    CaseStageDetail {
        case_id: id,
        current: entry
            .map(|e| CurrentCaseStage::Registered(Box::new(e)))
            .unwrap_or(CurrentCaseStage::Unregistered),
    }
}
pub fn support() -> Value {
    json!({"document_id": DOCUMENT, "version": 1, "digest": "ab".repeat(32)})
}
pub fn date() -> Value {
    json!({"precision":"date", "date":"2023-01-02", "offset":"-06:00"})
}
pub fn adoption() -> Value {
    json!({"expected_revision":0,"stage":"intermediate","known_at":date(),"reason":"  Motivo\r\n declarado  ","support":support()})
}
pub fn intermediate() -> Value {
    json!({"expected_revision":1,"target":"intermediate","accusation_declared_at":{"precision":"instant","at":"2023-02-01T10:11:12.125-06:00"},"accusation":support(),"note":" Nota "})
}
pub fn trial() -> Value {
    json!({"expected_revision":2,"target":"trial","opening_order_issued_at":date(),"opening_order":support(),"received_at":date(),"receiving_court":" Tribunal Uno ","receipt_reference":" Acuse 9 ","receipt_support":support(),"note":" Nota\r\nmanual "})
}

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub changes: Mutex<Vec<CaseStageChange>>,
}
impl Workflow {
    fn check(&self, token: &str) -> Result<(), ApplicationError> {
        let error = match token {
            "expired" => ApplicationError::InvalidSession,
            "forbidden" => ApplicationError::PermissionDenied,
            "hidden" => ApplicationError::CaseNotFound,
            "conflict" => ApplicationError::CaseStageConflict,
            "required" => ApplicationError::CaseStageRequired,
            "exhausted" => ApplicationError::CaseStageRevisionExhausted,
            "rejected" => ApplicationError::CaseStageTransitionRejected,
            "incomplete" => ApplicationError::CaseStageProfileIncomplete,
            "changed" => ApplicationError::StageSupportChanged,
            "digest" => ApplicationError::StageSupportDigestMismatch,
            "large" => ApplicationError::StageSupportTooLarge,
            "format" => ApplicationError::StageSupportFormatRejected,
            "limit" => ApplicationError::StageSupportValidationLimit,
            "closed" => ApplicationError::CaseClosed,
            "failed" => ApplicationError::Port("postgres://secret-password".into()),
            _ => return Ok(()),
        };
        Err(error)
    }
    fn changed(&self, id: CaseId, expected: u32, change: CaseStageChange) -> CaseStageDetail {
        self.changes.lock().unwrap().push(change.clone());
        let supports = change
            .supports()
            .into_iter()
            .map(|s| StageSupportSnapshot {
                reference: s.reference(),
                digest: s.digest(),
                name: "acta.pdf".into(),
                format: StageDocumentFormat::Pdf,
                policy: StageFormatPolicy::PdfDocxV1,
            })
            .collect();
        detail(
            id,
            Some(CaseStageEntry::Changed(Box::new(CaseStageSnapshot {
                case_id: id,
                stage_revision: CaseStageRevision::new(expected + 1).unwrap(),
                from_stage: match expected {
                    0 => None,
                    1 => Some(CaseStage::Investigation),
                    _ => Some(CaseStage::Intermediate),
                },
                values: change,
                values_digest: Sha256Digest::from_bytes(&[0x22; 32]).unwrap(),
                administration_revision: CaseRevision::FIRST,
                administration_digest: Sha256Digest::from_bytes(&[0x11; 32]).unwrap(),
                supports,
                recorded_at: at(),
                recorded_by: actor(),
            }))),
        )
    }
}
impl CaseStageWorkflow for Workflow {
    fn get(&self, token: &str, id: CaseId) -> Result<CaseStageDetail, ApplicationError> {
        self.check(token)?;
        self.calls
            .lock()
            .unwrap()
            .push(json!({"op":"get","case_id":id}));
        Ok(detail(
            id,
            if token == "empty" {
                None
            } else {
                Some(initial(if token == "foreign" {
                    CaseId::from_uuid(Uuid::from_u128(9))
                } else {
                    id
                }))
            },
        ))
    }
    fn history(
        &self,
        token: &str,
        id: CaseId,
        query: CaseStageQuery,
    ) -> Result<CaseStagePage, ApplicationError> {
        self.check(token)?;
        self.calls.lock().unwrap().push(json!({"op":"history","limit":query.limit(),"before":query.before_revision().map(|r|r.get())}));
        if token == "page" {
            let reference = domain::crypto::DocumentVersionRef {
                id: domain::crypto::DocumentId::from_uuid(Uuid::from_u128(2)),
                version: domain::crypto::DocumentVersion::initial(),
            };
            let change = CaseStageChange::Transition(StageTransition::to_intermediate(
                DeclaredStageTime::instant(at()).unwrap(),
                StageSupportRef::new(reference, Sha256Digest::from_bytes(&[0xab; 32]).unwrap()),
                None,
            ));
            let CurrentCaseStage::Registered(entry) = self.changed(id, 1, change).current else {
                unreachable!()
            };
            return Ok(CaseStagePage {
                entries: vec![*entry],
                has_more: true,
                next_before_revision: Some(CaseStageRevision::new(2).unwrap()),
            });
        }
        Ok(CaseStagePage {
            entries: vec![initial(id)],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn adopt(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseStageExpectation,
        values: StageAdoption,
    ) -> Result<CaseStageDetail, ApplicationError> {
        self.check(token)?;
        self.calls
            .lock()
            .unwrap()
            .push(json!({"op":"adopt","expected":expected.get(),"case_id":id}));
        Ok(self.changed(id, expected.get(), CaseStageChange::Adopt(values)))
    }
    fn transition(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseStageRevision,
        values: StageTransition,
    ) -> Result<CaseStageDetail, ApplicationError> {
        self.check(token)?;
        self.calls
            .lock()
            .unwrap()
            .push(json!({"op":"transition","expected":expected.get(),"case_id":id}));
        Ok(self.changed(id, expected.get(), CaseStageChange::Transition(values)))
    }
}
pub async fn raw(
    workflow: Arc<Workflow>,
    method: &str,
    suffix: &str,
    token: Option<&str>,
    body: String,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/api/v1/cases/{CASE}/stage{suffix}"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::case_stage_router(workflow)
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}
pub async fn call(
    workflow: Arc<Workflow>,
    method: &str,
    suffix: &str,
    token: &str,
    body: Value,
) -> Response {
    raw(workflow, method, suffix, Some(token), body.to_string()).await
}
pub async fn body(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap()).unwrap()
}
