#![allow(dead_code)]
use application::{alerts::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1788264000)
        .unwrap()
        .replace_nanosecond(123456789)
        .unwrap()
}
pub fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(1))
}
pub fn id() -> AlertId {
    AlertId::from_uuid(Uuid::from_u128(2))
}
pub fn record() -> AlertRecord {
    AlertRecord {
        id: id(),
        recipient_id: actor(),
        occurrence_id: AlertOccurrenceId::from_uuid(Uuid::from_u128(3)),
        subject: AlertSubject::Deadline {
            case_id: CaseId::from_uuid(Uuid::from_u128(4)),
            id: application::deadlines::DeadlineId::from_uuid(Uuid::from_u128(5)),
        },
        subject_title: "Captured deadline".into(),
        case_title: "Captured case".into(),
        case_reference: "CASE-4".into(),
        kind: AlertKind::OverdueUnattended {
            due_at: at() - Duration::hours(1),
        },
        origin: AlertOrigin {
            revision: 2,
            evidence_digest: Sha256Digest::from_array([7; 32]),
        },
        trigger_at: at() - Duration::hours(1),
        created_at: at(),
        read_at: None,
        state: AlertState::Active,
        email: AlertEmailStatus::Accepted { accepted_at: at() },
    }
}

pub struct Workflow {
    pub calls: AtomicUsize,
    pub denied: bool,
}
impl Workflow {
    fn call(&self, token: &str) -> Result<(), ApplicationError> {
        assert_eq!(token, "alert-session");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.denied {
            Err(ApplicationError::PermissionDenied)
        } else {
            Ok(())
        }
    }
}
impl AlertWorkflow for Workflow {
    fn preferences(&self, token: &str) -> Result<AlertPreferences, ApplicationError> {
        self.call(token)?;
        Ok(AlertPreferences::initial(
            actor(),
            AlertEmailTransport::Ready,
        ))
    }
    fn save_preferences(
        &self,
        token: &str,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError> {
        self.call(token)?;
        if command.expected_revision != 0 {
            return Err(AlertError::RevisionConflict.into());
        }
        Ok(AlertPreferences {
            user_id: actor(),
            revision: 1,
            values: command.values,
            updated_at: Some(at()),
            receipt: Some(AlertPreferenceReceipt {
                operation_id: command.operation_id,
                expected_revision: 0,
            }),
            email_transport: AlertEmailTransport::Ready,
        })
    }
    fn list(&self, token: &str, _query: AlertQuery) -> Result<AlertPage, ApplicationError> {
        self.call(token)?;
        Ok(AlertPage {
            checked_at: at(),
            alerts: vec![record()],
            has_more: false,
            next_cursor: None,
        })
    }
    fn get(&self, token: &str, requested: AlertId) -> Result<AlertDetail, ApplicationError> {
        self.call(token)?;
        if requested != id() {
            return Err(AlertError::NotFound.into());
        }
        Ok(AlertDetail {
            checked_at: at(),
            alert: record(),
        })
    }
    fn mark_read(
        &self,
        token: &str,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError> {
        self.call(token)?;
        if command.alert_id != id() {
            return Err(AlertError::NotFound.into());
        }
        let mut alert = record();
        alert.read_at = Some(at());
        Ok(AlertReadReceipt {
            operation_id: command.operation_id,
            checked_at: at(),
            alert,
        })
    }
}
pub fn setup(denied: bool) -> (Arc<Workflow>, Router) {
    let workflow = Arc::new(Workflow {
        calls: AtomicUsize::new(0),
        denied,
    });
    let router = web::alert_router(workflow.clone());
    (workflow, router)
}
pub async fn request(
    router: Router,
    method: &str,
    path: &str,
    body: Option<&str>,
    authenticated: bool,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if authenticated {
        builder = builder.header("authorization", "Bearer alert-session");
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let response = router
        .oneshot(
            builder
                .body(Body::from(body.unwrap_or("").to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 131072).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub fn preference_body() -> Value {
    let channels = json!({"internal":true,"email":true});
    let upcoming = json!({"lead_hours":[48,24],"channels":channels});
    json!({"operation_id":Uuid::from_u128(8).to_string(),"expected_revision":0,"values":{
        "hearing_upcoming":upcoming,"deadline_upcoming":upcoming,
        "overdue_unattended":channels,"review_required":channels,"due_changed_soon":channels
    }})
}
