#![allow(dead_code)]
#[path = "../resource_hearing_activity_support/mod.rs"]
mod captures;

use application::{resource_activities::*, resource_hearings::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, crypto::Sha256Digest, resource_hearings::*};
use mockall::mock;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

pub use captures::{case, digest, hearing_id, resource};
mock! {
    pub Write {}
    impl ResourceHearingWorkflow for Write {
        fn prepare(&self, token:&str, case:CaseId, resource:ResourceId, command:ResourceHearingCommand)->Result<ResourceHearingDraft,ApplicationError>;
        fn submit(&self, token:&str, case:CaseId, resource:ResourceId, command:ResourceHearingCommand, expected:Sha256Digest)->Result<ResourceHearingCreation,ApplicationError>;
    }
}
mock! {
    pub Read {}
    impl ResourceHearingReadWorkflow for Read {
        fn list(&self, token:&str, case:CaseId, resource:ResourceId, query:ResourceHearingReadQuery)->Result<ResourceHearingPage,ApplicationError>;
        fn get(&self, token:&str, case:CaseId, resource:ResourceId, hearing:ResourceHearingId, revision:Option<ResourceHearingRevision>)->Result<ResourceHearingCreation,ApplicationError>;
    }
}
pub fn base() -> String {
    format!("{}/resource-hearings", captures::base())
}
pub fn command() -> ResourceHearingCommand {
    captures::hearing().review.command
}
pub fn draft() -> ResourceHearingDraft {
    captures::hearing().review
}
pub fn creation() -> ResourceHearingCreation {
    let hearing = captures::hearing();
    let c = &hearing.review.command;
    let mut association = captures::detail();
    association.receipt.operation_id =
        ResourceActivityOperationId::from_uuid(c.operation_id.as_uuid());
    ResourceHearingCreation {
        origin: ResourceHearingOrigin {
            case_id: case(),
            resource_id: resource(),
            hearing_id: c.hearing_id,
            operation_id: c.operation_id,
            association_id: c.association_id,
            submission_digest: hearing.review.submission_digest,
            capture_digest: hearing.capture_digest,
        },
        hearing,
        association,
    }
}
pub fn page() -> ResourceHearingPage {
    ResourceHearingPage {
        case_id: case(),
        resource_id: resource(),
        items: vec![creation()],
        has_more: false,
        next_after_id: None,
    }
}
pub fn creation_with_people() -> ResourceHearingCreation {
    use application::{
        participants::*, procedural_facts::*, typed_participants::ParticipantOverview,
    };
    use domain::{hearings::HearingParticipantRef, procedural_facts::FactParticipantRef};
    let mut result = creation();
    let h = &mut result.hearing;
    let mut selected = Vec::new();
    for number in [11, 12] {
        let id = ParticipantId::from_uuid(uuid::Uuid::from_u128(number));
        let revision = ParticipantRevision::initial();
        let snapshot = ParticipantSnapshot {
            case_id: case(),
            id,
            revision,
            values: ParticipantValues::new(
                &format!("Person {number}"),
                "Declared role",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            values_digest: digest(),
            changed_at: captures::at(),
            changed_by: ParticipantActorSnapshot {
                id: h.review.recorded_by.id,
                email: format!("author{number}@example.test"),
            },
        };
        h.review.participants.push(FactParticipantProjection {
            snapshot: FactParticipantSnapshot {
                case_id: case(),
                reference: FactParticipantRef { id, revision },
                values_digest: digest(),
                status: DirectoryStatus::Active,
                subject: None,
            },
            overview: ParticipantOverview::from(&snapshot),
        });
        h.material.participants.push(snapshot.into());
        selected.push(HearingParticipantRef::new(id, revision));
    }
    let v = &h.review.command.values;
    h.review.command.values = ResourceHearingValues::new(ResourceHearingValuesInput {
        kind: v.kind(),
        scheduled_at: v.scheduled_at(),
        modality: v.modality(),
        venue: v.venue().clone(),
        note: v.note().cloned(),
        participants: selected,
        scheduling_basis: v.scheduling_basis().clone(),
    })
    .unwrap();
    result.association.sources.target =
        ResourceActivityTargetDetail::ResourceHearing(Box::new(h.clone()));
    result
}
pub fn body() -> Value {
    let c = command();
    let v = &c.values;
    let support = v.scheduling_basis().support();
    json!({"case_id":case(),"resource_id":resource().to_string(),
        "operation_id":c.operation_id.to_string(),"hearing_id":c.hearing_id.to_string(),
        "association_id":c.association_id.to_string(),"expected_resource_revision":5,
        "resource":{"id":resource().to_string(),"revision":2,"capture_digest":digest().to_hex()},"act":null,
        "values":{"kind":"written_revocation","scheduled_at":"1970-01-03T00:00:00Z",
            "modality":"videoconference","venue":v.venue().as_str(),"note":v.note().map(|n|n.as_str()),
            "participants":[],"scheduling_basis":{"statement":v.scheduling_basis().statement().as_str(),
                "support":{"document_id":support.reference().id.to_string(),"version":support.reference().version.get(),
                    "digest":support.digest().to_hex()}}}})
}
pub fn submission() -> Value {
    json!({"command":body(),"expected_submission_digest":digest().to_hex()})
}
pub async fn raw(
    w: MockWrite,
    r: MockRead,
    method: &str,
    path: &str,
    token: &str,
    body: String,
    content_type: &str,
) -> (u16, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", content_type);
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = web::resource_hearing_router(Arc::new(w), Arc::new(r))
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn request(
    w: MockWrite,
    r: MockRead,
    method: &str,
    path: &str,
    value: Option<Value>,
) -> (u16, Value) {
    raw(
        w,
        r,
        method,
        path,
        "owner",
        value.map(|v| v.to_string()).unwrap_or_default(),
        "application/json",
    )
    .await
}
pub fn internal() -> Value {
    json!({"error":{"code":"internal_error","message":"internal application error"}})
}
