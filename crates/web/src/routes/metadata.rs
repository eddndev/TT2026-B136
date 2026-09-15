//! Current classification and audited revision history within an authorized case.

use application::documents::MetadataQuery;
use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;

use super::{
    documents::{parse_case_id, parse_id},
    AppState,
};
use crate::{
    dto::{MetadataHistoryResponse, MetadataResponse, ReplaceMetadataRequest},
    error::ApiError,
    request::bearer_token,
};

pub(super) const MAX_METADATA_BYTES: usize = 8 * 1024;

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct HistoryQuery {
    limit: u32,
    before_revision: Option<u32>,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            limit: 50,
            before_revision: None,
        }
    }
}

pub(super) async fn get_metadata(
    State(state): State<AppState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<MetadataResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case_id(&case)?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let current = state
        .runtime
        .run(move || workflow.get_metadata(&token, case, id))
        .await?;
    Ok(Json(MetadataResponse::new(case, id, current)))
}

pub(super) async fn replace_metadata(
    State(state): State<AppState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Json<ReplaceMetadataRequest>, JsonRejection>,
) -> Result<Json<MetadataResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case_id(&case)?;
    let id = parse_id(&id)?;
    let Json(input) = input.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::payload_too_large("metadata_too_large", "metadata JSON exceeds 8 KiB")
        } else {
            ApiError::invalid_body("invalid_json", "invalid metadata JSON object")
        }
    })?;
    let (expected, metadata) = input.validate()?;
    let workflow = state.workflow.clone();
    let current = state
        .runtime
        .run(move || workflow.replace_metadata(&token, case, id, expected, metadata))
        .await?;
    Ok(Json(MetadataResponse::new(case, id, current)))
}

pub(super) async fn metadata_history(
    State(state): State<AppState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    Query(input): Query<HistoryQuery>,
) -> Result<Json<MetadataHistoryResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case_id(&case)?;
    let id = parse_id(&id)?;
    let query = MetadataQuery::new(input.limit, input.before_revision)?;
    let workflow = state.workflow.clone();
    let page = state
        .runtime
        .run(move || workflow.metadata_history(&token, case, id, query))
        .await?;
    Ok(Json(MetadataHistoryResponse::new(case, id, page)?))
}
