mod alert_http_support;
#[path = "alert_resource_hearing_http/precautionary.rs"]
mod precautionary;
use alert_http_support::*;
use application::{alerts::*, ApplicationError};
use axum::http::StatusCode;
use domain::{
    alerts::AlertLeadHours, procedural_resources::ResourceId, resource_hearings::ResourceHearingId,
};
use serde_json::{json, Value};
use std::sync::{atomic::AtomicUsize, Arc};
use time::Duration;
use uuid::Uuid;

struct ResourceHearingWorkflow {
    base: Workflow,
    alert: AlertRecord,
}
impl AlertWorkflow for ResourceHearingWorkflow {
    fn preferences(&self, token: &str) -> Result<AlertPreferences, ApplicationError> {
        self.base.preferences(token)
    }
    fn save_preferences(
        &self,
        token: &str,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError> {
        self.base.save_preferences(token, command)
    }
    fn list(&self, token: &str, query: AlertQuery) -> Result<AlertPage, ApplicationError> {
        let mut page = self.base.list(token, query)?;
        page.alerts = vec![self.alert.clone()];
        Ok(page)
    }
    fn get(&self, token: &str, id: AlertId) -> Result<AlertDetail, ApplicationError> {
        let mut detail = self.base.get(token, id)?;
        detail.alert = self.alert.clone();
        Ok(detail)
    }
    fn mark_read(
        &self,
        token: &str,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError> {
        let mut receipt = self.base.mark_read(token, command)?;
        receipt.alert = self.alert.clone();
        receipt.alert.read_at = Some(at());
        Ok(receipt)
    }
}
fn captured() -> AlertRecord {
    let mut value = record();
    value.subject = AlertSubject::ResourceHearing {
        case_id: value.subject.case_id(),
        resource_id: ResourceId::from_uuid(Uuid::from_u128(6)),
        id: ResourceHearingId::from_uuid(Uuid::from_u128(5)),
    };
    value.subject_title = "Captured resource hearing".into();
    value.origin.revision = 1;
    value.kind = AlertKind::Upcoming {
        lead_hours: AlertLeadHours::new(24).unwrap(),
        activity_at: at() + Duration::hours(24),
    };
    value.trigger_at = at();
    value.email = AlertEmailStatus::Disabled;
    value
}
fn router(alert: AlertRecord) -> axum::Router {
    web::alert_router(Arc::new(ResourceHearingWorkflow {
        base: Workflow {
            calls: AtomicUsize::new(0),
            denied: false,
        },
        alert,
    }))
}
fn subject() -> Value {
    json!({
        "kind":"resource_hearing", "case_id":Uuid::from_u128(4).to_string(),
        "resource_id":Uuid::from_u128(6).to_string(),"id":Uuid::from_u128(5).to_string()
    })
}

#[tokio::test]
async fn resource_hearing_inbox_and_detail_keep_parent_capture_and_exact_time() {
    let alert = captured();
    let router = router(alert.clone());
    let (status, page) = request(router.clone(), "GET", "/api/v1/alerts", None, true).await;
    assert_eq!(status, StatusCode::OK);
    let (status, detail) = request(
        router,
        "GET",
        &format!("/api/v1/alerts/{}", id()),
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["alerts"][0], detail["alert"]);
    let value = &detail["alert"];
    assert_eq!(value["subject"], subject());
    assert_eq!(value["origin"]["revision"], 1);
    assert_eq!(
        value["origin"]["evidence_digest"],
        alert.origin.evidence_digest.to_hex()
    );
    assert_eq!(value["kind"]["kind"], "upcoming");
    assert_eq!(value["kind"]["lead_hours"], 24);
    assert_eq!(
        value["kind"]["activity_at"]["unix_seconds"],
        (at() + Duration::hours(24)).unix_timestamp()
    );
    assert_eq!(value["kind"]["activity_at"]["nanosecond"], 123456789);
    assert_eq!(value["trigger_at"]["nanosecond"], 123456789);
    assert_eq!(value["email"], json!({"kind":"disabled"}));
    assert!(!value.to_string().contains("provider_id"));
}

#[tokio::test]
async fn resource_hearing_read_receipt_preserves_captured_origin_and_live_state() {
    let router = router(captured());
    let command = json!({"operation_id":Uuid::from_u128(9).to_string()});
    let (status, body) = request(
        router,
        "POST",
        &format!("/api/v1/alerts/{}/read", id()),
        Some(&command.to_string()),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["operation_id"], command["operation_id"]);
    assert_eq!(body["alert"]["subject"], subject());
    assert_eq!(
        body["alert"]["origin"]["evidence_digest"],
        captured().origin.evidence_digest.to_hex()
    );
    assert_eq!(body["alert"]["state"], json!({"kind":"active"}));
    assert_ne!(body["alert"]["read_at"], Value::Null);
}

#[tokio::test]
async fn resource_hearing_invalid_revision_or_non_upcoming_kind_fails_closed() {
    let mut wrong_revision = captured();
    wrong_revision.origin.revision = 2;
    let mut wrong_kind = captured();
    wrong_kind.kind = AlertKind::ReviewRequired;
    for alert in [wrong_revision, wrong_kind] {
        let router = router(alert);
        for path in [
            "/api/v1/alerts".to_owned(),
            format!("/api/v1/alerts/{}", id()),
        ] {
            let (status, body) = request(router.clone(), "GET", &path, None, true).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert!(body.get("alert").is_none());
            assert!(body.get("alerts").is_none());
        }
    }
}
