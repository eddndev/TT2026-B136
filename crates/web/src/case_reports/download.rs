use super::{input, query, response, ReportState};
use crate::{error::ApiError, request::bearer_token};
use application::case_reports::MAX_REPORT_ARTIFACT_BYTES;
use axum::{
    body::{Body, Bytes},
    extract::{rejection::QueryRejection, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use tokio::sync::OwnedSemaphorePermit;
use zeroize::Zeroizing;
struct RetainedReport {
    bytes: Zeroizing<Vec<u8>>,
    _permit: OwnedSemaphorePermit,
}
impl AsRef<[u8]> for RetainedReport {
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}
pub(super) async fn download(
    State(state): State<ReportState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    parameters: Result<Query<input::Download>, QueryRejection>,
) -> Result<Response, ApiError> {
    let token = bearer_token(&headers)?;
    let id = input::id(&id)?;
    let format = query(parameters)?.format()?;
    let permit = state.runtime.reserve_content()?;
    let (value, permit) = state
        .runtime
        .run(move || Ok((state.workflow.download(&token, id, format)?, permit)))
        .await?;
    let artifact = value.artifact;
    let bytes = Zeroizing::new(artifact.content);
    if artifact.report_id != id
        || value.report.id != id
        || artifact.format != format
        || bytes.len() > MAX_REPORT_ARTIFACT_BYTES
    {
        return Err(ApiError::internal());
    }
    let extension = response::format(format);
    let content_type = if extension == "pdf" {
        "application/pdf"
    } else {
        "text/csv; charset=utf-8"
    };
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .header(
            "content-disposition",
            format!("attachment; filename=\"report-{id}.{extension}\""),
        )
        .header("content-length", bytes.len())
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .header("x-report-id", id.to_string())
        .header("x-report-digest", artifact.digest.to_hex())
        .header(
            "x-report-snapshot-digest",
            artifact.snapshot_digest.to_hex(),
        )
        .body(Body::from(Bytes::from_owner(RetainedReport {
            bytes,
            _permit: permit,
        })))
        .map_err(|_| ApiError::internal())
}
