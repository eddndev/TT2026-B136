#![allow(dead_code)]
use application::{
    case_reports::*, cases::CaseStatusFilter, identity::Principal, ApplicationError,
};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use domain::{
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

pub const ID: &str = "00000000-0000-0000-0000-000000000009";
pub fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1790467200).unwrap()
}
pub fn request_body() -> Value {
    json!({"operation_id": ID,"filters": {
    "created_from":"2026-09-01T00:00:00Z","created_before":"2026-09-27T00:00:00Z",
    "status":"all","assigned_litigator":null }})
}
pub fn report(ready: bool) -> CaseReportDetail {
    let id = CaseReportId::from_uuid(Uuid::parse_str(ID).unwrap());
    let mut value = CaseReportDetail {
        id,
        requester: CaseReportRequester {
            principal: Principal {
                id: UserId::new(),
                email: "reporter@example.test".into(),
                role: Role::Owner,
            },
            account_revision: 4,
            auth_generation: 2,
        },
        scope: CaseReportScope::Office,
        command: CaseReportCommand {
            operation_id: CaseReportOperationId::from_uuid(id.as_uuid()),
            filters: CaseReportFilters {
                kind: CaseReportKind::CaseState,
                period_from: at() - time::Duration::days(26),
                period_before: at(),
                status: CaseStatusFilter::All,
                litigator: None,
            },
        },
        request_digest: Sha256Digest::from_array([1; 32]),
        requested_at: at(),
        updated_at: at(),
        state: CaseReportState::Queued,
        notice: None,
    };
    if ready {
        value.state = CaseReportState::Ready {
            snapshot_digest: Sha256Digest::from_array([2; 32]),
            checked_at: at(),
            artifacts: vec![
                CaseReportArtifactMetadata {
                    format: CaseReportFormat::Pdf,
                    bytes: pdf().len() as u64,
                    digest: Sha256Digest::from_array([3; 32]),
                },
                CaseReportArtifactMetadata {
                    format: CaseReportFormat::Csv,
                    bytes: csv().len() as u64,
                    digest: Sha256Digest::from_array([4; 32]),
                },
            ],
        };
        value.notice = Some(CaseReportNotice {
            kind: CaseReportNoticeKind::Ready,
            created_at: at(),
            read_at: None,
        });
    }
    value
}
pub fn pdf() -> Vec<u8> {
    b"%PDF-1.7\n\x00\xffliteral bytes\n%%EOF\n".to_vec()
}
pub fn csv() -> Vec<u8> {
    b"row_type,title\r\ncase,'Title\r\n".to_vec()
}
#[derive(Debug, PartialEq)]
pub enum Call {
    Request(CaseReportCommand),
    List(CaseReportQuery),
    Litigators(CaseReportLitigatorQuery),
    Get(CaseReportId),
    Ack(CaseReportId),
    Download(CaseReportId, CaseReportFormat),
}
pub struct Workflow {
    pub calls: Mutex<Vec<Call>>,
    pub ready: bool,
    pub failure: Option<&'static str>,
}
impl Workflow {
    fn call(&self, token: &str, call: Call) -> Result<CaseReportDetail, ApplicationError> {
        assert_eq!(token, "report-session");
        self.calls.lock().unwrap().push(call);
        let error = match self.failure {
            Some("not_found") => CaseReportError::NotFound,
            Some("operation_conflict") => CaseReportError::OperationConflict,
            Some("not_ready") => CaseReportError::NotReady,
            Some("capacity_exceeded") => CaseReportError::CapacityExceeded,
            Some("access_revoked") => CaseReportError::AccessRevoked,
            Some("render_unavailable") => CaseReportError::RenderUnavailable,
            Some("render_failed") => CaseReportError::RenderFailed,
            Some("stored_inconsistent") => {
                CaseReportError::StoredInconsistent("private database payload".into())
            }
            Some("denied") => return Err(ApplicationError::PermissionDenied),
            Some("port") => return Err(ApplicationError::Port("private database payload".into())),
            _ => return Ok(report(self.ready)),
        };
        Err(error.into())
    }
}
impl CaseReportWorkflow for Workflow {
    fn request(&self, t: &str, c: CaseReportCommand) -> Result<CaseReportDetail, ApplicationError> {
        let mut report = self.call(t, Call::Request(c.clone()))?;
        report.command = c;
        Ok(report)
    }
    fn list(&self, t: &str, q: CaseReportQuery) -> Result<CaseReportPage, ApplicationError> {
        let value = self.call(t, Call::List(q))?;
        Ok(CaseReportPage {
            checked_at: at(),
            reports: vec![value],
            has_more: false,
            next_after_id: None,
        })
    }
    fn litigators(
        &self,
        t: &str,
        query: CaseReportLitigatorQuery,
    ) -> Result<CaseReportLitigatorPage, ApplicationError> {
        let value = self.call(t, Call::Litigators(query))?;
        let member = UserId::from_uuid(Uuid::parse_str(ID).unwrap());
        Ok(CaseReportLitigatorPage {
            scope: value.scope,
            checked_at: at(),
            litigators: vec![CaseReportLitigator {
                user_id: member,
                email: "lawyer@example.test".into(),
            }],
            has_more: query.limit == 1,
            next_after_id: (query.limit == 1).then_some(member),
        })
    }
    fn get(&self, t: &str, id: CaseReportId) -> Result<CaseReportDetail, ApplicationError> {
        self.call(t, Call::Get(id))
    }
    fn acknowledge_notice(
        &self,
        t: &str,
        id: CaseReportId,
    ) -> Result<CaseReportDetail, ApplicationError> {
        let mut value = self.call(t, Call::Ack(id))?;
        value.notice.as_mut().unwrap().read_at = Some(at());
        Ok(value)
    }
    fn download(
        &self,
        t: &str,
        id: CaseReportId,
        f: CaseReportFormat,
    ) -> Result<CaseReportDownload, ApplicationError> {
        let value = self.call(t, Call::Download(id, f))?;
        Ok(CaseReportDownload {
            artifact: CaseReportArtifact {
                report_id: id,
                requester: value.requester.clone(),
                scope: value.scope,
                format: f,
                snapshot_digest: Sha256Digest::from_array([2; 32]),
                digest: Sha256Digest::from_array([3; 32]),
                content: match f {
                    CaseReportFormat::Pdf => pdf(),
                    CaseReportFormat::Csv => csv(),
                },
            },
            report: value,
        })
    }
}
pub fn setup(ready: bool, failure: Option<&'static str>) -> (Arc<Workflow>, Router) {
    let workflow = Arc::new(Workflow {
        calls: Mutex::new(vec![]),
        ready,
        failure,
    });
    (workflow.clone(), web::case_reports_router(workflow))
}
pub async fn send(
    router: Router,
    method: &str,
    path: &str,
    body: Option<Value>,
    auth: bool,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/api/v1/case-reports{path}"));
    if auth {
        request = request.header("Authorization", "Bearer report-session");
    }
    let bytes = body
        .map(|v| serde_json::to_vec(&v).unwrap())
        .unwrap_or_default();
    if !bytes.is_empty() {
        request = request.header("Content-Type", "application/json");
    }
    let response = router
        .oneshot(request.body(Body::from(bytes)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 32 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    (status, headers, bytes)
}
pub fn body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
pub fn private(headers: &HeaderMap) {
    assert_eq!(headers.get("cache-control").unwrap(), "no-store");
}
