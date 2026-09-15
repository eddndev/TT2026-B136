//! Authorized metadata queries with bounded pagination and literal name search.

use application::documents::{DocumentMetadataFilter, DocumentQuery};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::documents::{parse_case_id, parse_id};
use super::AppState;
use crate::{dto::DocumentOverviewResponse, error::ApiError, request::bearer_token};

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ListDocuments {
    limit: u32,
    offset: u32,
    name: Option<String>,
    sealed: Option<bool>,
    document_type: Option<String>,
    classification: Option<String>,
    tag: Option<String>,
}

impl Default for ListDocuments {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
            name: None,
            sealed: None,
            document_type: None,
            classification: None,
            tag: None,
        }
    }
}

#[derive(Serialize)]
pub(super) struct DocumentPageResponse {
    documents: Vec<DocumentOverviewResponse>,
    has_more: bool,
}

pub(super) async fn list_documents(
    State(state): State<AppState>,
    Path(case_id): Path<String>,
    headers: HeaderMap,
    Query(input): Query<ListDocuments>,
) -> Result<Json<DocumentPageResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let query = DocumentQuery::new(
        input.limit,
        input.offset,
        input.name.as_deref(),
        input.sealed,
    )?
    .with_metadata_filter(DocumentMetadataFilter::new(
        input.document_type.as_deref(),
        input.classification.as_deref(),
        input.tag.as_deref(),
    )?);
    let workflow = state.workflow.clone();
    let page = state
        .runtime
        .run(move || workflow.list(&token, case_id, query))
        .await?;
    Ok(Json(DocumentPageResponse {
        documents: page.documents.into_iter().map(Into::into).collect(),
        has_more: page.has_more,
    }))
}

pub(super) async fn get_document(
    State(state): State<AppState>,
    Path((case_id, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<DocumentOverviewResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let case_id = parse_case_id(&case_id)?;
    let id = parse_id(&id)?;
    let workflow = state.workflow.clone();
    let summary = state
        .runtime
        .run(move || workflow.get(&token, case_id, id))
        .await?;
    Ok(Json(summary.into()))
}
