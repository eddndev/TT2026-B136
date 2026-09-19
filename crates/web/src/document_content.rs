//! Fully validated exact content is exposed only after workflow authorization.

use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::document_content::{DocumentContent, DocumentContentWorkflow};
use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, RawQuery, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;

#[derive(Clone)]
struct ContentState {
    workflow: Arc<dyn DocumentContentWorkflow>,
    runtime: HttpRuntime,
}

struct RetainedContent {
    content: DocumentContent,
    _permit: OwnedSemaphorePermit,
}

impl AsRef<[u8]> for RetainedContent {
    fn as_ref(&self) -> &[u8] {
        self.content.bytes.as_slice()
    }
}

pub(crate) fn router(workflow: Arc<dyn DocumentContentWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(
            "/api/v1/cases/:case/documents/:id/versions/:version/content",
            get(content).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
        )
        .layer(DefaultBodyLimit::max(0))
        .with_state(ContentState { workflow, runtime })
}

async fn content(
    State(state): State<ContentState>,
    Path((case, id, version)): Path<(String, String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = bearer_token(&headers)?;
    if query.is_some_and(|value| !value.is_empty()) {
        return Err(ApiError::invalid_body(
            "invalid_content_query",
            "content requires an exact path without query parameters",
        ));
    }
    let case = CaseId::from_uuid(case.parse().map_err(|_| ApiError::invalid_case_id())?);
    let id = DocumentId::from_uuid(id.parse().map_err(|_| ApiError::invalid_document_id())?);
    let number: u32 = version
        .parse()
        .map_err(|_| ApiError::invalid_document_version())?;
    if number.to_string() != version {
        return Err(ApiError::invalid_document_version());
    }
    let version = DocumentVersion::new(number).map_err(|_| ApiError::invalid_document_version())?;
    let reference = DocumentVersionRef { id, version };
    let workflow = state.workflow;
    let permit = state.runtime.reserve_content()?;
    let (result, permit) = state
        .runtime
        .run(move || {
            let result = workflow.content_version(&token, case, reference)?;
            Ok((result, permit))
        })
        .await?;
    if result.case_id != case
        || result.reference != reference
        || result.bytes.len() > 16 * 1024 * 1024
    {
        return Err(ApiError::internal());
    }
    let filename = safe_filename(&result.file_name);
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(|_| ApiError::invalid_response_header())?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .header("content-disposition", disposition)
        .header("content-length", result.bytes.len())
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .header("x-case-id", case.to_string())
        .header("x-document-id", id.to_string())
        .header("x-document-version", version.get())
        .header("x-document-digest", result.digest.to_hex())
        // Bytes retains the zeroizing content and its permit until every copy is
        // released, including chunks retained by the transport after body EOF.
        .body(Body::from(Bytes::from_owner(RetainedContent {
            content: result,
            _permit: permit,
        })))
        .map_err(|_| ApiError::internal())?;
    Ok(response)
}

fn safe_filename(name: &str) -> String {
    let name: String = name
        .chars()
        .take(120)
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_' | ' ') {
                value
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim_matches([' ', '.']);
    if name.is_empty() {
        "document.bin".to_owned()
    } else {
        name.to_owned()
    }
}
