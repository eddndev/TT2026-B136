#![allow(dead_code)]
#[path = "../procedural_resource_http_support/model.rs"]
pub(super) mod resources;
use application::{resource_activities::*, resource_hearings::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, crypto::Sha256Digest, hearings::*, resource_hearings::*};
use mockall::mock;
use serde_json::{json, Value};
use std::sync::Arc;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

mock! {
    pub Workflow {}
    impl ResourceActivityWorkflow for Workflow {
        fn list_for_target(&self, token:&str, case:CaseId, target:ResourceActivityTargetId, query:ResourceActivityTargetQuery)->Result<ResourceActivityTargetPage,ApplicationError>;
        fn list(&self, token:&str, case:CaseId, resource:ResourceId, query:ResourceActivityQuery)->Result<ResourceActivityPage,ApplicationError>;
        fn get(&self, token:&str, case:CaseId, resource:ResourceId, id:ResourceActivityId, revision:Option<ResourceActivityRevision>)->Result<ResourceActivityView,ApplicationError>;
        fn history(&self, token:&str, case:CaseId, resource:ResourceId, id:ResourceActivityId, query:ResourceActivityHistoryQuery)->Result<ResourceActivityHistoryPage,ApplicationError>;
        fn prepare(&self, token:&str, case:CaseId, resource:ResourceId, command:ResourceActivityCommand)->Result<ResourceActivityDraft,ApplicationError>;
        fn submit(&self, token:&str, case:CaseId, resource:ResourceId, command:ResourceActivityCommand, expected:Sha256Digest)->Result<ResourceActivityDetail,ApplicationError>;
    }
}
pub fn case() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn resource() -> ResourceId {
    ResourceId::from_uuid(Uuid::from_u128(2))
}
pub fn id() -> ResourceActivityId {
    ResourceActivityId::from_uuid(Uuid::from_u128(3))
}
pub fn hearing_id() -> ResourceHearingId {
    ResourceHearingId::from_uuid(Uuid::from_u128(7))
}
pub fn digest() -> Sha256Digest {
    resources::digest()
}
pub fn at() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}
pub fn base() -> String {
    format!(
        "/api/v1/cases/{}/procedural-resources/{}/activities",
        case(),
        resource()
    )
}
pub fn hearing() -> ResourceHearingDetail {
    let source = resources::detail(case(), &resources::command(resource(), 2));
    let head = resources::detail(case(), &resources::command(resource(), 5));
    let support = source.sources.supports[0].clone();
    let review = ResourceHearingDraft {
        case_id: case(),
        command: ResourceHearingCommand {
            operation_id: ResourceHearingOperationId::from_uuid(Uuid::from_u128(9)),
            hearing_id: hearing_id(),
            association_id: id(),
            expected_resource_revision: head.revision,
            resource: ResourceCaptureRef {
                id: resource(),
                revision: source.revision,
                capture_digest: source.receipt.capture_digest,
            },
            act: None,
            values: ResourceHearingValues::new(ResourceHearingValuesInput {
                kind: ResourceHearingKind::WrittenRevocation,
                scheduled_at: HearingTime::new(at() + time::Duration::days(2)).unwrap(),
                modality: HearingModality::Videoconference,
                venue: HearingVenue::new("Declared room").unwrap(),
                note: Some(HearingNote::new("Declared appointment").unwrap()),
                participants: vec![],
                scheduling_basis: ResourceHearingSchedulingBasis::new(
                    HearingNote::new("Previously admitted scheduling order").unwrap(),
                    HearingSupportRef::new(support.reference, support.digest),
                ),
            })
            .unwrap(),
        },
        resource: source.clone(),
        act: None,
        support,
        participants: vec![],
        observed_administration: head.recorded_administration.clone(),
        observed_resource_head: ResourceCaptureRef {
            id: resource(),
            revision: head.revision,
            capture_digest: head.receipt.capture_digest,
        },
        recorded_by: source.recorded_by.clone(),
        submission_digest: digest(),
    };
    ResourceHearingDetail {
        material: ResourceHearingMaterial {
            case_id: case(),
            administration: head.recorded_administration.clone(),
            resource_head: head,
            resource: source,
            act: None,
            participants: vec![],
        },
        review,
        revision: ResourceHearingRevision::initial(),
        recorded_at: at(),
        capture_digest: digest(),
    }
}
pub fn selection() -> ResourceActivitySelection {
    ResourceActivitySelection {
        resource: hearing().review.command.resource,
        act: None,
        target: ResourceActivityTarget::ResourceHearing {
            id: hearing_id(),
            revision: ResourceHearingRevision::initial(),
            capture_digest: digest(),
        },
    }
}
pub fn command() -> ResourceActivityCommand {
    ResourceActivityCommand {
        operation_id: ResourceActivityOperationId::from_uuid(Uuid::from_u128(4)),
        association_id: id(),
        expected_resource_revision: ResourceRevision::new(5).unwrap(),
        change: ResourceActivityChange::Link {
            selection: selection(),
        },
    }
}
pub fn draft() -> ResourceActivityDraft {
    let hearing = hearing();
    ResourceActivityDraft {
        case_id: case(),
        resource_id: resource(),
        command: command(),
        result_revision: ResourceActivityRevision::initial(),
        selection: selection(),
        status: ResourceActivityStatus::Linked,
        sources: ResourceActivitySources {
            resource: hearing.review.resource.clone(),
            act: None,
            target: ResourceActivityTargetDetail::ResourceHearing(Box::new(hearing.clone())),
        },
        previous: None,
        recorded_by: hearing.review.recorded_by,
        observed_administration: hearing.review.observed_administration,
        observed_resource_head: hearing.review.observed_resource_head,
        submission_digest: digest(),
    }
}
pub fn detail() -> ResourceActivityDetail {
    let draft = draft();
    ResourceActivityDetail {
        case_id: case(),
        resource_id: resource(),
        id: id(),
        revision: draft.result_revision,
        selection: draft.selection,
        status: draft.status,
        sources: draft.sources,
        reason: None,
        receipt: ResourceActivityReceipt {
            operation_id: command().operation_id,
            action: ResourceActivityAction::Link,
            expected_revision: 0,
            expected_resource_revision: command().expected_resource_revision,
            previous: None,
            submission_digest: digest(),
            capture_digest: digest(),
        },
        recorded_by: draft.recorded_by,
        recorded_at: at(),
        recorded_administration: draft.observed_administration,
        recorded_resource_head: draft.observed_resource_head,
    }
}
pub fn view() -> ResourceActivityView {
    ResourceActivityView {
        association: detail(),
        checked_at: at(),
        current_target: ResourceActivityCurrentTarget::ResourceHearing(Box::new(hearing())),
    }
}
pub fn body() -> Value {
    json!({"case_id":case(),"resource_id":resource().to_string(),"association_id":id().to_string(),
        "operation_id":command().operation_id.to_string(),"expected_resource_revision":5,
        "change":{"action":"link","expected_revision":0,"act":null,
            "resource":{"id":resource().to_string(),"revision":2,"capture_digest":digest().to_hex()},
            "target":{"kind":"resource_hearing","id":hearing_id().to_string(),
                "revision":1,"capture_digest":digest().to_hex()}}})
}
pub async fn request(
    workflow: MockWorkflow,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let response = web::resource_activity_router(Arc::new(workflow))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", "Bearer owner")
                .header("content-type", "application/json")
                .body(
                    body.map(|v| Body::from(v.to_string()))
                        .unwrap_or_else(Body::empty),
                )
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
