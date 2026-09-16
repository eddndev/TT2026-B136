use application::{judicial_calendars::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;
pub const BASE: &str = "/api/v1/judicial-calendars";
pub const ID: &str = "00000000-0000-0000-0000-000000000000";
pub fn id() -> JudicialCalendarId {
    JudicialCalendarId::from_uuid(Uuid::nil())
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x77; 32])
}
pub fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(99))
}
pub fn vector(name: &str) -> Value {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../domain/tests/fixtures/judicial_calendar_vectors.json"
    ))
    .unwrap();
    rows.into_iter().find(|v| v["name"] == name).unwrap()
}
pub fn values() -> JudicialCalendarValues {
    let v = vector("leap_unicode_unordered");
    let bytes: Vec<u8> = v["hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect();
    JudicialCalendarValues::from_canonical_bytes(&bytes).unwrap()
}
pub fn command_json() -> Value {
    json!({"operation_id":ID,"calendar_id":ID,"change":{"action":"publish","expected_revision":0,"values":vector("leap_unicode_unordered")["input"]}})
}
pub fn command() -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::from_uuid(Uuid::nil()),
        calendar_id: id(),
        change: JudicialCalendarChange::Publish { values: values() },
    }
}
pub fn detail(c: &JudicialCalendarCommand) -> JudicialCalendarDetail {
    JudicialCalendarDetail {
        id: c.calendar_id,
        revision: c.result_revision().unwrap(),
        values: match &c.change {
            JudicialCalendarChange::Publish { values }
            | JudicialCalendarChange::Replace { values, .. } => values.clone(),
            _ => values(),
        },
        values_digest: digest(),
        status: c.result_status(),
        reason: c.reason().cloned(),
        receipt: JudicialCalendarReceipt {
            operation_id: c.operation_id,
            action: c.action(),
            expected_revision: c.expected_revision(),
            submission_digest: digest(),
        },
        recorded_at: OffsetDateTime::UNIX_EPOCH,
        recorded_by: JudicialCalendarActorSnapshot {
            id: actor(),
            email: "owner@example.test".into(),
        },
    }
}
#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub response: Mutex<Option<JudicialCalendarDetail>>,
}
impl Workflow {
    fn record(&self, token: &str, call: Value) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(call);
        match token {
            "client" | "denied" => Err(ApplicationError::PermissionDenied),
            "expired" => Err(ApplicationError::InvalidSession),
            "missing" => Err(JudicialCalendarError::NotFound.into()),
            "revision" => Err(JudicialCalendarError::RevisionConflict.into()),
            "operation" => Err(JudicialCalendarError::OperationConflict.into()),
            "retired" => Err(JudicialCalendarError::Retired.into()),
            "exhausted" => Err(JudicialCalendarError::RevisionExhausted.into()),
            "scope" => Err(JudicialCalendarError::ScopeChangeForbidden.into()),
            "mismatch" => Err(JudicialCalendarError::SubmissionMismatch.into()),
            "internal" => Err(JudicialCalendarError::StoredInconsistent(
                "secret SQL and source".into(),
            )
            .into()),
            _ => Ok(()),
        }
    }
}
impl JudicialCalendarWorkflow for Workflow {
    fn list(
        &self,
        token: &str,
        q: JudicialCalendarQuery,
    ) -> Result<JudicialCalendarPage, ApplicationError> {
        self.record(
            token,
            json!([
                "list",
                q.limit(),
                q.after_id().map(|v| v.to_string()),
                q.status().status().map(|s| s.as_str()),
                q.jurisdiction().map(|j| j.as_str()),
                q.entity_code()
            ]),
        )?;
        Ok(JudicialCalendarPage {
            calendars: vec![JudicialCalendarOverview::from(&detail(&command()))],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        token: &str,
        id: JudicialCalendarId,
        revision: Option<JudicialCalendarRevision>,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        self.record(
            token,
            json!(["get", id.to_string(), revision.map(|r| r.get())]),
        )?;
        Ok(self
            .response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| detail(&command())))
    }
    fn history(
        &self,
        token: &str,
        id: JudicialCalendarId,
        q: JudicialCalendarHistoryQuery,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError> {
        self.record(
            token,
            json!([
                "history",
                id.to_string(),
                q.limit(),
                q.before_revision().map(|v| v.get())
            ]),
        )?;
        Ok(JudicialCalendarHistoryPage {
            revisions: vec![JudicialCalendarHistoryEntry::from(&detail(&command()))],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn days(
        &self,
        token: &str,
        id: JudicialCalendarId,
        revision: JudicialCalendarRevision,
        q: JudicialCalendarDaysQuery,
    ) -> Result<JudicialCalendarDays, ApplicationError> {
        self.record(
            token,
            json!([
                "days",
                id.to_string(),
                revision.get(),
                q.from().to_string(),
                q.through().to_string()
            ]),
        )?;
        let values = values();
        let days = (q.from().days_since_epoch()..=q.through().days_since_epoch())
            .map(|d| values.classify(CivilDate::from_days_since_epoch(d).unwrap()))
            .collect();
        Ok(JudicialCalendarDays {
            calendar_id: id,
            revision,
            values_digest: digest(),
            days,
        })
    }
    fn prepare(
        &self,
        token: &str,
        c: JudicialCalendarCommand,
    ) -> Result<JudicialCalendarDraft, ApplicationError> {
        self.record(
            token,
            json!(["prepare", c.calendar_id.to_string(), c.action().as_str()]),
        )?;
        let d = detail(&c);
        Ok(JudicialCalendarDraft {
            actor: actor(),
            result_revision: c.result_revision()?,
            command: c,
            initial_scope: d.values.scope().clone(),
            values: d.values,
            values_digest: digest(),
            submission_digest: digest(),
        })
    }
    fn submit(
        &self,
        token: &str,
        c: JudicialCalendarCommand,
        expected: Sha256Digest,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "submit",
                c.calendar_id.to_string(),
                c.action().as_str(),
                expected.to_hex()
            ]),
        )?;
        Ok(self
            .response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| detail(&c)))
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
    let response = web::judicial_calendar_router(w)
        .oneshot(
            r.body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
pub async fn raw(text: String) -> (u16, Value, usize) {
    let w = Arc::new(Workflow::default());
    let (status, body) = request(
        w.clone(),
        "POST",
        &format!("{BASE}/prepare"),
        Some("owner"),
        Some(text),
        &["application/json"],
    )
    .await;
    let calls = w.calls.lock().unwrap().len();
    (status, body, calls)
}
pub fn mutation(action: &str) -> (&'static str, String, Value) {
    let mut c = command_json();
    let (method, suffix) = match action {
        "prepare" => return ("POST", format!("{BASE}/prepare"), c),
        "publish" => ("POST", String::new()),
        "replace" => ("PUT", format!("/{ID}")),
        "retire" => ("POST", format!("/{ID}/retirement")),
        _ => panic!("unsupported action"),
    };
    if action != "publish" {
        c["change"] =
            json!({"action":action,"expected_revision":1,"reason":" Explicit reason\r\nMore "});
        if action == "replace" {
            c["change"]["values"] = vector("leap_unicode_unordered")["input"].clone();
        }
    }
    (
        method,
        format!("{BASE}{suffix}"),
        json!({"command":c,"expected_submission_digest":digest().to_hex()}),
    )
}
