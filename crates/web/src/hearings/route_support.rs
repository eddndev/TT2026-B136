use application::{hearings::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) const CASE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
pub(super) const ID: &str = "00000000-0000-4000-8000-000000000012";
pub(super) fn case() -> CaseId {
    CaseId::from_uuid(Uuid::parse_str(CASE).unwrap())
}
pub(super) fn id() -> HearingId {
    HearingId::from_uuid(Uuid::parse_str(ID).unwrap())
}
pub(super) fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(99))
}
pub(super) fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x66; 32])
}
pub(super) fn values() -> HearingValues {
    serde_json::from_value::<super::values::Values>(super::value_tests::values())
        .unwrap()
        .validate()
        .unwrap()
}
pub(super) fn command() -> HearingCommand {
    serde_json::from_value::<super::request::Command>(super::value_tests::command())
        .unwrap()
        .validate()
        .unwrap()
}
pub(super) fn context() -> HearingCaseContext {
    use application::{case_stages::CurrentCaseStage, cases::CurrentCaseAdministration};
    HearingCaseContext {
        case_id: case(),
        administration: CurrentCaseAdministration::Unrevised(
            domain::cases::CaseMetadata::new("Test case", "H-001").unwrap(),
        ),
        stage: CurrentCaseStage::Unregistered,
    }
}
pub(super) fn detail() -> HearingDetail {
    use domain::{
        case_administration::{CaseRevision, CaseStageRevision},
        case_stages::CaseStage,
    };
    HearingDetail {
        snapshot: HearingSnapshot {
            case_id: case(),
            id: id(),
            revision: HearingRevision::initial(),
            values: values(),
            values_digest: digest(),
            status: HearingStatus::Scheduled,
            reason: None,
            receipt: HearingReceipt {
                operation_id: command().operation_id,
                action: HearingAction::Schedule,
                expected_revision: 0,
                expected_context: Some(HearingContextExpectation {
                    case_revision: CaseRevision::FIRST,
                    stage_revision: CaseStageRevision::FIRST,
                }),
                submission_digest: digest(),
            },
            scheduling_context: HearingSchedulingContext {
                administration_revision: CaseRevision::FIRST,
                administration_digest: digest(),
                stage_revision: CaseStageRevision::FIRST,
                stage: CaseStage::Investigation,
                stage_digest: None,
            },
            recorded_administration_revision: CaseRevision::FIRST,
            recorded_administration_digest: digest(),
            recorded_at: OffsetDateTime::UNIX_EPOCH,
            recorded_by: application::cases::CaseActorSnapshot {
                id: actor(),
                email: "captured@example.test".into(),
            },
        },
        participants: vec![],
        support: None,
    }
}
pub(super) fn overview() -> HearingOverview {
    let d = detail();
    HearingOverview {
        case_id: case(),
        case_title: "Test case".into(),
        case_reference: "H-001".into(),
        case_status: domain::case_administration::CaseAdministrativeStatus::Active,
        id: id(),
        revision: d.snapshot.revision,
        kind: d.snapshot.values.kind(),
        scheduled_at: d.snapshot.values.scheduled_at(),
        modality: d.snapshot.values.modality(),
        status: d.snapshot.status,
        participant_count: 0,
    }
}
#[derive(Default)]
pub(super) struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub response: Mutex<Option<HearingDetail>>,
}
impl Workflow {
    fn record(&self, token: &str, value: Value) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(value);
        match token {
            "forbidden" => Err(ApplicationError::PermissionDenied),
            "conflict" => Err(HearingError::RevisionConflict.into()),
            "missing" => Err(HearingError::NotFound.into()),
            "expired" => Err(ApplicationError::InvalidSession),
            "internal" => {
                Err(HearingError::StoredInconsistent("secret venue and SQL".into()).into())
            }
            _ => Ok(()),
        }
    }
}
impl HearingWorkflow for Workflow {
    fn context(&self, token: &str, case: CaseId) -> Result<HearingCaseContext, ApplicationError> {
        self.record(token, json!(["context", case]))?;
        Ok(context())
    }
    fn list(
        &self,
        token: &str,
        case: CaseId,
        q: HearingQuery,
    ) -> Result<HearingPage, ApplicationError> {
        self.record(token, json!(["list", case, q.limit()]))?;
        Ok(HearingPage {
            hearings: vec![overview()],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        id: HearingId,
        revision: Option<HearingRevision>,
    ) -> Result<HearingDetail, ApplicationError> {
        self.record(
            token,
            json!(["get", case, id.to_string(), revision.map(|r| r.get())]),
        )?;
        Ok(self.response.lock().unwrap().clone().unwrap_or_else(detail))
    }
    fn history(
        &self,
        token: &str,
        case: CaseId,
        id: HearingId,
        q: HearingHistoryQuery,
    ) -> Result<HearingHistoryPage, ApplicationError> {
        self.record(token, json!(["history", case, id.to_string(), q.limit()]))?;
        Ok(HearingHistoryPage {
            revisions: vec![detail()],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn agenda(
        &self,
        token: &str,
        q: HearingAgendaQuery,
    ) -> Result<HearingAgendaPage, ApplicationError> {
        self.record(
            token,
            json!([
                "agenda",
                q.from().unix_timestamp(),
                q.until().unix_timestamp()
            ]),
        )?;
        Ok(HearingAgendaPage {
            hearings: vec![overview()],
            has_more: false,
            next_after: None,
        })
    }
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        command: HearingCommand,
    ) -> Result<HearingDraft, ApplicationError> {
        self.record(
            token,
            json!(["prepare", case, command.hearing_id.to_string()]),
        )?;
        Ok(HearingDraft {
            command,
            actor: actor(),
            submission_digest: digest(),
            result_revision: HearingRevision::initial(),
            values: values(),
            values_digest: digest(),
        })
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        command: HearingCommand,
        expected: Sha256Digest,
    ) -> Result<HearingDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "submit",
                case,
                command.hearing_id.to_string(),
                expected.to_hex()
            ]),
        )?;
        let mut d = detail();
        match command.change {
            HearingChange::Schedule { .. } => {}
            HearingChange::Replace {
                expected_revision,
                values,
                reason,
                ..
            } => {
                d.snapshot.revision = expected_revision.next().unwrap();
                d.snapshot.values = values;
                d.snapshot.reason = Some(reason);
                d.snapshot.receipt.action = HearingAction::Replace;
                d.snapshot.receipt.expected_revision = expected_revision.get();
            }
            HearingChange::Cancel {
                expected_revision,
                reason,
            } => {
                d.snapshot.revision = expected_revision.next().unwrap();
                d.snapshot.status = HearingStatus::Cancelled;
                d.snapshot.reason = Some(reason);
                d.snapshot.receipt.action = HearingAction::Cancel;
                d.snapshot.receipt.expected_revision = expected_revision.get();
                d.snapshot.receipt.expected_context = None;
            }
        }
        Ok(d)
    }
}
pub(super) async fn call(
    w: Arc<Workflow>,
    method: &str,
    path: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (u16, Value) {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        r = r.header("authorization", format!("Bearer {token}"));
    }
    let r = r
        .body(body.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    let response = crate::hearing_router(w).oneshot(r).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
