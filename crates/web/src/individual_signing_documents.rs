use super::{conflict, input, response, token};
use crate::{error::ApiError, runtime::HttpRuntime};
use application::document_signing::DocumentSigningService;
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    routing::{get, post},
    Json, Router,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
};
use serde_json::Value;
use std::sync::Arc;
use zeroize::Zeroizing;

const PREFIX: &str = "/api/v1/cases/:case/documents/:document/versions/:version/individual-signing";
type ReferencePath = (String, String, String);
#[derive(Clone)]
struct DocumentState {
    service: Arc<DocumentSigningService>,
    runtime: HttpRuntime,
}

pub(super) fn router(service: Arc<DocumentSigningService>, runtime: HttpRuntime) -> Router {
    Router::new()
        .route(&format!("{PREFIX}/prepare"), post(prepare))
        .route(&format!("{PREFIX}/submit"), post(submit))
        .route(&format!("{PREFIX}/receipts/:operation"), get(receipt))
        .route(&format!("{PREFIX}/verification"), get(verification))
        .layer(DefaultBodyLimit::max(input::BODY_LIMIT))
        .with_state(DocumentState { service, runtime })
}
fn admission(
    raw: ReferencePath,
    request: &Request,
) -> Result<(Zeroizing<String>, CaseId, DocumentVersionRef), ApiError> {
    let token = token(request)?;
    let case = CaseId::from_uuid(input::uuid(&raw.0)?);
    let id = DocumentId::from_uuid(input::uuid(&raw.1)?);
    if raw.2.is_empty() || !raw.2.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(input::invalid());
    }
    let version = DocumentVersion::new(raw.2.parse().map_err(|_| input::invalid())?)
        .map_err(|_| input::invalid())?;
    Ok((token, case, DocumentVersionRef { id, version }))
}
async fn prepare(
    State(state): State<DocumentState>,
    Path(raw): Path<ReferencePath>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case, reference) = admission(raw, &request)?;
    let (operation, signature) = input::read::<input::DocumentPreparation>(request)
        .await?
        .values()?;
    let row = state
        .runtime
        .run(move || {
            state
                .service
                .prepare(&token, operation, case, reference, &signature)
        })
        .await?;
    Ok(Json(response::document_preparation(row)))
}
async fn submit(
    State(state): State<DocumentState>,
    Path(raw): Path<ReferencePath>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case, reference) = admission(raw, &request)?;
    let value = input::read::<input::Submission>(request).await?.values()?;
    if value.consent.case() != case || value.consent.document() != reference {
        return Err(conflict());
    }
    let row = state
        .runtime
        .run(move || {
            state.service.submit(
                &token,
                &value.consent,
                &value.pdf_signature,
                &value.consent_signature,
            )
        })
        .await?;
    Ok(Json(response::receipt(row)?))
}
async fn receipt(
    State(state): State<DocumentState>,
    Path(raw): Path<(String, String, String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case, reference) = admission((raw.0, raw.1, raw.2), &request)?;
    let operation = input::uuid(&raw.3)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.get(&token, case, reference, operation))
        .await?;
    Ok(Json(response::receipt(row)?))
}
async fn verification(
    State(state): State<DocumentState>,
    Path(raw): Path<ReferencePath>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let (token, case, reference) = admission(raw, &request)?;
    input::empty(request).await?;
    let row = state
        .runtime
        .run(move || state.service.verify(&token, case, reference))
        .await?;
    Ok(Json(response::verification(row)?))
}
