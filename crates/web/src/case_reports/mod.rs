//! Durable report requests and byte-exact authorized downloads.
mod download;
mod input;
mod response;
use crate::{error::ApiError, request::bearer_token, runtime::HttpRuntime};
use application::case_reports::{CaseReportState, CaseReportWorkflow};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
#[derive(Clone)]
struct ReportState {
    workflow: Arc<dyn CaseReportWorkflow>,
    runtime: HttpRuntime,
}
pub(crate) fn router(workflow: Arc<dyn CaseReportWorkflow>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route("/api/v1/case-reports", get(list).post(request))
        .route("/api/v1/case-reports/litigators", get(litigators))
        .route("/api/v1/case-reports/:id", get(detail))
        .route("/api/v1/case-reports/:id/notice-read", post(acknowledge))
        .route(
            "/api/v1/case-reports/:id/download",
            get(download::download).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
        )
        .with_state(ReportState { workflow, runtime })
}
fn query<T>(value: Result<Query<T>, QueryRejection>) -> Result<T, ApiError> {
    value.map(|Query(v)| v).map_err(|_| input::invalid())
}
async fn request(
    State(state): State<ReportState>,
    parameters: Result<Query<input::Empty>, QueryRejection>,
    request: Request,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let token = bearer_token(request.headers())?;
    query(parameters)?;
    let body: input::RequestBody = crate::request::json::read(
        request,
        4096,
        "case_report_request_too_large",
        "report request exceeds 4096 bytes",
    )
    .await?;
    let command = body.command()?;
    let value = state
        .runtime
        .run(move || state.workflow.request(&token, command))
        .await?;
    let status = if matches!(
        value.state,
        CaseReportState::Ready { .. } | CaseReportState::Failed(_) | CaseReportState::AccessRevoked
    ) {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    Ok((status, Json(response::detail(value)?)))
}
async fn list(
    State(state): State<ReportState>,
    headers: HeaderMap,
    parameters: Result<Query<input::Page>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let parameters = query(parameters)?.query()?;
    let value = state
        .runtime
        .run(move || state.workflow.list(&token, parameters))
        .await?;
    Ok(Json(
        json!({"checked_at":response::at(value.checked_at)?,"reports":value.reports.into_iter().map(response::detail).collect::<Result<Vec<_>,_>>()?,
        "has_more":value.has_more,"next_after_id":value.next_after_id.map(|id|id.to_string())}),
    ))
}
async fn litigators(
    State(state): State<ReportState>,
    headers: HeaderMap,
    parameters: Result<Query<input::Litigators>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let parameters = query(parameters)?.query()?;
    let value = state
        .runtime
        .run(move || state.workflow.litigators(&token, parameters))
        .await?;
    Ok(Json(response::litigators(value)?))
}
async fn detail(
    State(state): State<ReportState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    parameters: Result<Query<input::Empty>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    query(parameters)?;
    let id = input::id(&id)?;
    let value = state
        .runtime
        .run(move || state.workflow.get(&token, id))
        .await?;
    Ok(Json(response::detail(value)?))
}
async fn acknowledge(
    State(state): State<ReportState>,
    Path(id): Path<String>,
    parameters: Result<Query<input::Empty>, QueryRejection>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(request.headers())?;
    query(parameters)?;
    let id = input::id(&id)?;
    let _: input::Empty = crate::request::json::read(
        request,
        4096,
        "case_report_request_too_large",
        "report request exceeds 4096 bytes",
    )
    .await?;
    let value = state
        .runtime
        .run(move || state.workflow.acknowledge_notice(&token, id))
        .await?;
    Ok(Json(response::detail(value)?))
}
