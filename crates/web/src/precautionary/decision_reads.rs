use super::{decision_router::DecisionState, measure_request::invalid, primitives::*, projection};
use crate::{error::ApiError, request::bearer_token};
use application::precautionary_measures::*;
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use domain::{
    cases::CaseId,
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<String>,
    after_id: Option<String>,
}
impl Page {
    fn validate(self) -> Result<MeasureDecisionReadQuery, ApiError> {
        let limit = self.limit.map(|v| number(&v)).transpose()?.unwrap_or(10);
        MeasureDecisionReadQuery::new(
            u16::try_from(limit).map_err(|_| invalid())?,
            self.after_id
                .map(|v| uuid(&v).map(MeasureDecisionId::from_uuid))
                .transpose()?,
        )
        .map_err(|_| invalid())
    }
}

pub(super) async fn list(
    State(s): State<DecisionState>,
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
                let mut last = query.after_id();
                for item in &page.items {
                    let id = item.origin().decision_id;
                    if last.is_some_and(|prior| prior.as_uuid() >= id.as_uuid()) {
                        return Err(ApiError::internal());
                    }
                    projection::bound_decision(item, case, Some(id), None)?;
                    last = Some(id);
                }
                if page.next_after_id != if page.has_more { last } else { None } {
                    return Err(ApiError::internal());
                }
                let items = page
                    .items
                    .iter()
                    .map(|v| projection::decision_operation(v, s.hasher.as_ref()))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(
                    json!({"case_id":case.to_string(),"items":items,"has_more":page.has_more,
                "next_after_id":page.next_after_id.map(|id|id.to_string())}),
                )
            })())
        })
        .await??;
    Ok(Json(value))
}

pub(super) async fn detail(
    State(s): State<DecisionState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let id = MeasureDecisionId::from_uuid(uuid(&id)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let receipt = s.reads.get(&token, case, id)?;
                projection::bound_decision(&receipt, case, Some(id), None)?;
                projection::decision_operation(&receipt, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}

pub(super) async fn operation(
    State(s): State<DecisionState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let id = MeasureDecisionOperationId::from_uuid(uuid(&id)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let receipt = s.reads.get_operation(&token, case, id)?;
                projection::bound_decision(&receipt, case, None, Some(id))?;
                projection::decision_operation(&receipt, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
