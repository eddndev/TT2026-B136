use super::{query, DeadlineState, RoutePath};
use crate::{error::ApiError, request::bearer_token};
use application::deadlines::*;
use axum::{
    extract::{rejection::QueryRejection, Path, Query, State},
    http::HeaderMap,
    Json,
};
use domain::identity::UserId;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResponsibleQuery {
    limit: Option<String>,
    after_id: Option<String>,
}
impl ResponsibleQuery {
    fn validate(self) -> Result<DeadlineResponsibleQuery, ApiError> {
        let limit = self
            .limit
            .as_deref()
            .map(|value| super::request::parse_u32(value).ok_or_else(query::invalid))
            .unwrap_or(Ok(20))?;
        let after = self
            .after_id
            .map(|value| super::request::uuid(&value, "invalid_query").map(UserId::from_uuid))
            .transpose()?;
        DeadlineResponsibleQuery::new(limit, after).map_err(|_| query::invalid())
    }
}

pub(super) async fn list(
    State(state): State<DeadlineState>,
    Path(path): Path<RoutePath>,
    headers: HeaderMap,
    input: Result<Query<ResponsibleQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = path.case_id()?;
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let value = state.runtime.run(move || Ok((|| {
        let page = state.workflow.responsibles(&token, case, query)?;
        page.validate(case, query).map_err(|_| ApiError::internal())?;
        let responsibles: Vec<_> = page.responsibles.into_iter().map(|row| {
            json!({"id":row.id,"email":row.email,"role":row.role.as_str()})
        }).collect();
        Ok::<_,ApiError>(json!({"case_id":case.to_string(),"responsibles":responsibles,"has_more":page.has_more,"next_after_id":page.next_after_id}))
    })())).await??;
    Ok(Json(value))
}
