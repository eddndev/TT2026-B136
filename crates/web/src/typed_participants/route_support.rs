use super::{proposal::Preparation, proposal_tests::preparation, values::SubjectValues};
use application::{typed_participants::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    response::Response,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest, identity::UserId};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

pub(super) const CASE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
pub(super) const SUBJECT: &str = "22222222-2222-4222-8222-222222222222";
pub(super) const PARTICIPANT: &str = "33333333-3333-4333-8333-333333333333";
pub(super) fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::parse_str(CASE).unwrap())
}
pub(super) fn subject_values() -> Value {
    json!({"kind":"natural_person","name":{"state":"known","value":"Captured Ana"},
        "curp":{"state":"unknown","reason":"Missing"},
        "identity_support":{"document_id":"11111111-1111-4111-8111-111111111111",
            "version":1,"digest":"11".repeat(32),"locator":"Page 1"}})
}
pub(super) fn subject_snapshot() -> SubjectSnapshot {
    SubjectSnapshot {
        case_id: case_id(),
        id: CaseSubjectId::from_uuid(Uuid::parse_str(SUBJECT).unwrap()),
        revision: SubjectRevision::initial(),
        values: serde_json::from_value::<SubjectValues>(subject_values())
            .unwrap()
            .validate()
            .unwrap(),
        values_digest: Sha256Digest::from_array([0x22; 32]),
        changed_at: OffsetDateTime::UNIX_EPOCH,
        changed_by: ParticipantActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(7)),
            email: "historical@example.test".into(),
        },
    }
}
pub(super) fn prepared() -> ParticipantPreparationRequest {
    serde_json::from_value::<Preparation>(preparation())
        .unwrap()
        .validate()
        .unwrap()
}
pub(super) fn detail() -> ParticipantDetail {
    let subject = subject_snapshot();
    ParticipantDetail {
        revision: ParticipantRevisionSnapshot::Typed(Box::new(TypedParticipantSnapshot {
            case_id: case_id(),
            id: ParticipantId::from_uuid(Uuid::parse_str(PARTICIPANT).unwrap()),
            revision: ParticipantRevision::initial(),
            values: prepared().proposal.values().clone(),
            values_digest: Sha256Digest::from_array([0x55; 32]),
            changed_at: subject.changed_at,
            changed_by: subject.changed_by.clone(),
            credential_origin: None,
            submission_digest: Sha256Digest::from_array([0x66; 32]),
            submission_revision: ParticipantRevision::initial(),
        })),
        bound_subject: Some(subject),
    }
}
#[derive(Default)]
pub(super) struct Workflow {
    pub calls: Mutex<Vec<Value>>,
    pub evidence: Mutex<Option<ParticipantCredentialEvidence>>,
}
impl Workflow {
    fn record(&self, token: &str, value: Value) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(value);
        match token {
            "forbidden" => Err(ApplicationError::PermissionDenied),
            "review-changed" => Err(ApplicationError::ParticipantIdentityReviewConflict),
            "revoked" => Err(domain::crypto::CredentialFailure::Revoked.into()),
            "internal" => Err(ApplicationError::Port("secret DSN or identity".into())),
            _ => Ok(()),
        }
    }
}
impl TypedParticipantWorkflow for Workflow {
    fn review_participant(
        &self,
        token: &str,
        case: CaseId,
        request: ParticipantProposalRequest,
    ) -> Result<ParticipantProposalReview, ApplicationError> {
        self.record(
            token,
            json!([
                "review",
                case.to_string(),
                request.role.kind().as_str(),
                request.certificate
            ]),
        )?;
        Ok(ParticipantProposalReview {
            proposal: prepared().proposal,
            directory_stamp: CaseDirectoryStamp(Sha256Digest::from_array([0x44; 32])),
            candidates: vec![],
        })
    }
    fn prepare_participant(
        &self,
        token: &str,
        case: CaseId,
        request: ParticipantPreparationRequest,
    ) -> Result<ParticipantSigningDraft, ApplicationError> {
        self.record(
            token,
            json!([
                "prepare",
                case.to_string(),
                request.proposal.participant_id().to_string(),
                request.certificate
            ]),
        )?;
        Ok(ParticipantSigningDraft {
            proposal: request.proposal,
            review: request.review,
            declaration: None,
            submission_digest: Some(Sha256Digest::from_array([0x66; 32])),
            submission_revision: ParticipantRevision::initial(),
        })
    }
    fn submit_participant(
        &self,
        token: &str,
        case: CaseId,
        request: ParticipantSubmission,
    ) -> Result<ParticipantDetail, ApplicationError> {
        self.record(
            token,
            json!([
                "commit",
                case.to_string(),
                request.prepared.proposal.participant_id().to_string(),
                request.signature.map(|v| v.as_bytes().len())
            ]),
        )?;
        Ok(detail())
    }
    fn review_subject(
        &self,
        token: &str,
        case: CaseId,
        id: CaseSubjectId,
        expected: SubjectRevision,
        values: application::typed_participants::SubjectValues,
    ) -> Result<SubjectReplacementReview, ApplicationError> {
        self.record(
            token,
            json!([
                "review_subject",
                case.to_string(),
                id.to_string(),
                expected.get()
            ]),
        )?;
        Ok(SubjectReplacementReview {
            id,
            expected_revision: expected,
            values,
            directory_stamp: CaseDirectoryStamp(Sha256Digest::from_array([0x44; 32])),
            candidates: vec![],
        })
    }
    fn replace_subject(
        &self,
        token: &str,
        case: CaseId,
        request: SubjectReplacementRequest,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        self.record(
            token,
            json!([
                "replace_subject",
                case.to_string(),
                request.id.to_string(),
                request.expected_revision.get()
            ]),
        )?;
        Ok(subject_snapshot())
    }
    fn list_subjects(
        &self,
        token: &str,
        case: CaseId,
        query: SubjectQuery,
    ) -> Result<SubjectPage, ApplicationError> {
        self.record(
            token,
            json!([
                "subjects",
                case.to_string(),
                query.limit(),
                query.after_id().map(|v| v.to_string()),
                query.name(),
                query.kind().map(|v| v.as_str())
            ]),
        )?;
        let s = subject_snapshot();
        Ok(SubjectPage {
            subjects: vec![SubjectOverview {
                case_id: case,
                id: s.id,
                revision: s.revision,
                kind: s.values.kind(),
                display_name: s.values.display_name().into(),
            }],
            has_more: true,
            next_after_id: Some(s.id),
        })
    }
    fn get_subject(
        &self,
        token: &str,
        case: CaseId,
        id: CaseSubjectId,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        self.record(token, json!(["subject", case.to_string(), id.to_string()]))?;
        Ok(subject_snapshot())
    }
    fn get_subject_revision(
        &self,
        token: &str,
        case: CaseId,
        id: CaseSubjectId,
        revision: SubjectRevision,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        self.record(
            token,
            json!([
                "subject_revision",
                case.to_string(),
                id.to_string(),
                revision.get()
            ]),
        )?;
        Ok(subject_snapshot())
    }
    fn subject_history(
        &self,
        token: &str,
        case: CaseId,
        id: CaseSubjectId,
        query: SubjectHistoryQuery,
    ) -> Result<SubjectHistoryPage, ApplicationError> {
        self.record(
            token,
            json!([
                "subject_history",
                case.to_string(),
                id.to_string(),
                query.limit(),
                query.before_revision().map(|v| v.get())
            ]),
        )?;
        Ok(SubjectHistoryPage {
            revisions: vec![subject_snapshot()],
            has_more: false,
            next_before_revision: None,
        })
    }
    fn credential(
        &self,
        token: &str,
        case: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
    ) -> Result<ParticipantCredentialEvidence, ApplicationError> {
        self.record(
            token,
            json!([
                "credential",
                case.to_string(),
                id.to_string(),
                revision.get()
            ]),
        )?;
        self.evidence
            .lock()
            .unwrap()
            .clone()
            .ok_or(ApplicationError::ParticipantCredentialNotFound)
    }
}
pub(super) async fn request(
    w: &Arc<Workflow>,
    method: &str,
    suffix: &str,
    token: Option<&str>,
    body: impl Into<Body>,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/api/v1/cases/{CASE}/{suffix}"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    crate::typed_participant_router(w.clone())
        .oneshot(request.body(body.into()).unwrap())
        .await
        .unwrap()
}
pub(super) async fn body(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 512 * 1024).await.unwrap()).unwrap()
}
