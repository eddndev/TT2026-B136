use super::{measure_request::invalid, primitives::*, projection, AdministrativeState};
use crate::{error::ApiError, request::bearer_token};
use application::measure_corrections::*;
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use domain::{cases::CaseId, precautionary_measures::MeasureCorrectionOperationId};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<String>,
    after_operation_id: Option<String>,
}
impl Page {
    fn validate(self) -> Result<MeasureAdministrativeReadQuery, ApiError> {
        let limit = self.limit.map(|v| number(&v)).transpose()?.unwrap_or(10);
        MeasureAdministrativeReadQuery::new(
            u16::try_from(limit).map_err(|_| invalid())?,
            self.after_operation_id
                .map(|v| uuid(&v).map(MeasureCorrectionOperationId::from_uuid))
                .transpose()?,
        )
        .map_err(|_| invalid())
    }
}
pub(super) async fn list(
    State(s): State<AdministrativeState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    input: Result<Query<Page>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let query = input.map_err(|_| invalid())?.0.validate()?;
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let page = s.reads.list(&token, case, query)?;
                if page.case_id != case
                    || page.items.len() > usize::from(query.limit())
                    || (page.has_more && page.items.len() != usize::from(query.limit()))
                {
                    return Err(ApiError::internal());
                }
                let mut last = query.after_operation_id();
                let mut items = Vec::with_capacity(page.items.len());
                for item in &page.items {
                    let id = item.origin.operation_id;
                    if last.is_some_and(|prior| prior.as_uuid() >= id.as_uuid()) {
                        return Err(ApiError::internal());
                    }
                    projection::bound_admin(item, case, Some(id))?;
                    last = Some(id);
                    items.push(projection::admin_operation(item, s.hasher.as_ref())?);
                }
                if page.next_after_operation_id != if page.has_more { last } else { None } {
                    return Err(ApiError::internal());
                }
                Ok(
                    json!({"case_id":case.to_string(),"items":items,"has_more":page.has_more,
            "next_after_operation_id":page.next_after_operation_id.map(|id|id.to_string())}),
                )
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn operation(
    State(s): State<AdministrativeState>,
    Path((case, operation)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let operation = MeasureCorrectionOperationId::from_uuid(uuid(&operation)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let value = s.reads.get_operation(&token, case, operation)?;
                projection::bound_admin(&value, case, Some(operation))?;
                projection::admin_operation(&value, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
