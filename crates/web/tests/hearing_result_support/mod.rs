use application::{
    cases::{CaseActorSnapshot, CaseAdministrationSnapshot, CurrentCaseAdministration},
    hearing_results::*,
    ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    case_stages::CaseStage,
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::{HearingId, HearingKind, HearingRevision, HearingStatus, HearingTime},
    identity::UserId,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;
pub const CASE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
pub const HEARING: &str = "00000000-0000-4000-8000-000000000012";
pub const RESULT: &str = "00000000-0000-4000-8000-000000000013";
pub fn case() -> CaseId {
    CaseId::from_uuid(Uuid::parse_str(CASE).unwrap())
}
pub fn hearing() -> HearingId {
    HearingId::from_uuid(Uuid::parse_str(HEARING).unwrap())
}
pub fn result() -> HearingResultId {
    HearingResultId::from_uuid(Uuid::parse_str(RESULT).unwrap())
}
pub fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(99))
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x66; 32])
}
pub fn base_url() -> String {
    format!("/api/v1/cases/{CASE}/hearings/{HEARING}/results")
}
pub fn values_json() -> Value {
    json!({"occurrence":"occurred","extent":"partial","event_time":{"precision":"date","date":"2026-09-16","offset":"-06:00"},"summary":" Declared session\r\nSummary ","attendees":[],"agreements":[],"provenance":{"kind":"operator_note","reference":null,"support":null}})
}
pub fn command_json() -> Value {
    json!({"operation_id":"00000000-0000-4000-8000-000000000014","hearing_id":HEARING,"result_id":RESULT,"change":{"action":"record","expected_revision":0,"anchor_revision":2,"continuation":null,"values":values_json()}})
}
pub fn values() -> HearingResultValues {
    HearingResultValues::new(HearingResultValuesInput {
        occurrence: HearingResultOccurrence::Occurred,
        extent: HearingResultExtent::Partial,
        event_time: DeclaredHearingResultTime::date(
            time::Date::from_calendar_date(2026, time::Month::September, 16).unwrap(),
            time::UtcOffset::from_hms(-6, 0, 0).unwrap(),
        )
        .unwrap(),
        summary: HearingResultText::new("Declared session\nSummary").unwrap(),
        attendees: vec![],
        agreements: vec![],
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            None,
        )
        .unwrap(),
    })
    .unwrap()
}
pub fn command() -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::from_uuid(
            Uuid::parse_str("00000000-0000-4000-8000-000000000014").unwrap(),
        ),
        hearing_id: hearing(),
        result_id: result(),
        change: HearingResultChange::Record {
            anchor_revision: HearingRevision::new(2).unwrap(),
            continuation: None,
            values: values(),
        },
    }
}
pub fn anchor() -> HearingResultAnchorSnapshot {
    HearingResultAnchorSnapshot {
        reference: HearingResultAnchor {
            hearing_id: hearing(),
            revision: HearingRevision::new(2).unwrap(),
            values_digest: digest(),
            submission_digest: digest(),
        },
        status: HearingStatus::Cancelled,
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(OffsetDateTime::UNIX_EPOCH).unwrap(),
        scheduling_context: application::hearings::HearingSchedulingContext {
            administration_revision: CaseRevision::FIRST,
            administration_digest: digest(),
            stage_revision: CaseStageRevision::FIRST,
            stage: CaseStage::Investigation,
            stage_digest: None,
        },
    }
}
pub fn administration() -> CurrentCaseAdministration {
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case(),
        revision: CaseRevision::FIRST,
        values: application::cases::CaseAdministrationValues::basic(
            CaseMetadata::new("Case", "REF").unwrap(),
        ),
        values_digest: digest(),
        changed_at: OffsetDateTime::UNIX_EPOCH,
        changed_by: CaseActorSnapshot {
            id: actor(),
            email: "actor@example.test".into(),
        },
    }))
}
pub fn detail(command: &HearingResultCommand) -> HearingResultDetail {
    let values = match &command.change {
        HearingResultChange::Record { values, .. }
        | HearingResultChange::Correct { values, .. } => values.clone(),
        _ => values(),
    };
    HearingResultDetail {
        snapshot: HearingResultSnapshot {
            case_id: case(),
            hearing_id: command.hearing_id,
            id: command.result_id,
            revision: command.result_revision().unwrap(),
            values,
            values_digest: digest(),
            status: if command.action() == HearingResultAction::Withdraw {
                HearingResultStatus::Withdrawn
            } else {
                HearingResultStatus::Recorded
            },
            reason: command.reason().cloned(),
            receipt: HearingResultReceipt {
                operation_id: command.operation_id,
                action: command.action(),
                expected_revision: command.expected_revision(),
                submission_digest: digest(),
            },
            anchor: anchor().reference,
            continuation: None,
            recorded_administration_revision: CaseRevision::FIRST,
            recorded_administration_digest: digest(),
            recorded_at: OffsetDateTime::UNIX_EPOCH,
            recorded_by: CaseActorSnapshot {
                id: actor(),
                email: "actor@example.test".into(),
            },
        },
        anchor: anchor(),
        continuation: None,
        attendees: match &command.change {
            HearingResultChange::Record { values, .. }
            | HearingResultChange::Correct { values, .. } => values
                .attendees()
                .iter()
                .map(|a| HearingResultAttendeeSnapshot {
                    participant: application::hearings::HearingParticipantSnapshot {
                        overview: application::participants::ParticipantOverview {
                            case_id: case(),
                            id: a.participant_id(),
                            revision: a.revision(),
                            display_name: "Declared attendee".into(),
                            procedural_role: "Declared role".into(),
                            organization: None,
                            directory_status: application::participants::DirectoryStatus::Archived,
                            kind: None,
                            subject: None,
                        },
                        values_digest: digest(),
                    },
                    subject_digest: None,
                })
                .collect(),
            _ => vec![],
        },
        support: match &command.change {
            HearingResultChange::Record { values, .. }
            | HearingResultChange::Correct { values, .. } => {
                values.provenance().support().map(|s| {
                    application::case_stages::StageSupportSnapshot {
                        reference: s.reference(),
                        digest: s.digest(),
                        name: "support.pdf".into(),
                        format: application::documents::StageDocumentFormat::Pdf,
                        policy: application::documents::StageFormatPolicy::PdfDocxV1,
                    }
                })
            }
            _ => None,
        },
    }
}
#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub response: Mutex<Option<HearingResultDetail>>,
}
impl Workflow {
    fn record(&self, token: &str, value: Value) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(value);
        match token {
            "forbidden" => Err(ApplicationError::PermissionDenied),
            "expired" => Err(ApplicationError::InvalidSession),
            "missing" => Err(HearingResultError::NotFound.into()),
            "source_missing" => Err(HearingResultError::ReferenceNotFound.into()),
            "case_missing" => Err(ApplicationError::CaseNotFound),
            "conflict" => Err(HearingResultError::RevisionConflict.into()),
            "future" => Err(HearingResultError::FutureTime.into()),
            "internal" => {
                Err(HearingResultError::StoredInconsistent("secret source and SQL".into()).into())
            }
            _ => Ok(()),
        }
    }
}
impl HearingResultWorkflow for Workflow {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        hearing: HearingId,
        q: HearingResultQuery,
    ) -> Result<HearingResultPage, ApplicationError> {
        self.record(
            token,
            json!([
                "list",
                case,
                hearing.to_string(),
                q.limit(),
                q.after_id().map(|id| id.to_string()),
                q.status().status().map(|s| s.as_str())
            ]),
        )?;
        Ok(HearingResultPage {
            results: vec![HearingResultOverview {
                case_id: case,
                hearing_id: hearing,
                id: result(),
                revision: HearingResultRevision::initial(),
                status: HearingResultStatus::Recorded,
                occurrence: HearingResultOccurrence::Occurred,
                extent: HearingResultExtent::Partial,
                event_time: values().event_time(),
                attendee_count: 0,
                agreement_count: 0,
                anchor_revision: HearingRevision::new(2).unwrap(),
            }],
            has_more: false,
            next_after_id: None,
        })
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        revision: Option<HearingResultRevision>,
    ) -> Result<HearingResultDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "get",
                case,
                hearing.to_string(),
                id.to_string(),
                revision.map(|r| r.get())
            ]),
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
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        q: HearingResultHistoryQuery,
    ) -> Result<HearingResultHistoryPage, ApplicationError> {
        self.record(
            token,
            json!([
                "history",
                case,
                hearing.to_string(),
                id.to_string(),
                q.limit(),
                q.before_revision().map(|r| r.get())
            ]),
        )?;
        Ok(HearingResultHistoryPage {
            revisions: vec![HearingResultHistoryEntry::from(
                &detail(&command()).snapshot,
            )],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        command: HearingResultCommand,
    ) -> Result<HearingResultDraft, ApplicationError> {
        self.record(
            token,
            json!([
                "prepare",
                case,
                command.hearing_id.to_string(),
                command.result_id.to_string()
            ]),
        )?;
        let detail = detail(&command);
        Ok(HearingResultDraft {
            case_id: case,
            actor: actor(),
            result_revision: command.result_revision()?,
            command,
            values: detail.snapshot.values,
            values_digest: digest(),
            submission_digest: digest(),
            anchor: anchor(),
            continuation: None,
            observed_administration: administration(),
            attendees: detail.attendees,
            support: detail.support,
        })
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        command: HearingResultCommand,
        expected: Sha256Digest,
    ) -> Result<HearingResultDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "submit",
                case,
                command.hearing_id.to_string(),
                command.result_id.to_string(),
                command.action().as_str(),
                expected.to_hex()
            ]),
        )?;
        Ok(detail(&command))
    }
}
pub async fn request(
    workflow: Arc<Workflow>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<String>,
    types: &[&str],
) -> (u16, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    for value in types {
        request = request.header("content-type", *value);
    }
    let response = web::hearing_result_router(workflow)
        .oneshot(
            request
                .body(body.map(Body::from).unwrap_or_else(Body::empty))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
