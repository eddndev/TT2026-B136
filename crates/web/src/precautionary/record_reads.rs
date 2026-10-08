use super::{measure_request::invalid, primitives::*, projection, AdministrativeState};
use crate::{error::ApiError, request::bearer_token};
use application::measure_corrections::*;
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use domain::{
    cases::CaseId,
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
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
    fn validate(self) -> Result<MeasureRecordReadQuery, ApiError> {
        let limit = self.limit.map(|v| number(&v)).transpose()?.unwrap_or(10);
        MeasureRecordReadQuery::new(
            u16::try_from(limit).map_err(|_| invalid())?,
            self.after_id
                .map(|v| uuid(&v).map(MeasureId::from_uuid))
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
                let page = s.records.list(&token, case, query)?;
                if page.case_id != case
                    || page.items.len() > usize::from(query.limit())
                    || (page.has_more && page.items.len() != usize::from(query.limit()))
                {
                    return Err(ApiError::internal());
                }
                let mut last = query.after_id();
                let mut items = Vec::with_capacity(page.items.len());
                for item in &page.items {
                    let id = item.reference.id();
                    if last.is_some_and(|prior| prior.as_uuid() >= id.as_uuid()) {
                        return Err(ApiError::internal());
                    }
                    projection::bound_record(item, case, Some(id), None)?;
                    last = Some(id);
                    items.push(projection::record_detail(item, s.hasher.as_ref())?);
                }
                if page.next_after_id != if page.has_more { last } else { None } {
                    return Err(ApiError::internal());
                }
                Ok(
                    json!({"case_id":case.to_string(),"items":items,"has_more":page.has_more,
            "next_after_id":page.next_after_id.map(|id|id.to_string())}),
                )
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn current(
    State(s): State<AdministrativeState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let id = MeasureId::from_uuid(uuid(&id)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let value = s.records.get(&token, case, id)?;
                projection::bound_record(&value, case, Some(id), None)?;
                projection::record_detail(&value, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Exact {
    capture_digest: String,
}
pub(super) async fn exact(
    State(s): State<AdministrativeState>,
    Path((case, id, revision)): Path<(String, String, String)>,
    headers: HeaderMap,
    input: Result<Query<Exact>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let id = MeasureId::from_uuid(uuid(&id)?);
    let reference = PrecautionaryMeasureRef::new(
        id,
        MeasureRevision::new(number(&revision)?).map_err(|_| invalid())?,
        digest(&input.map_err(|_| invalid())?.0.capture_digest)?,
    );
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let value = s.records.exact(&token, case, reference)?;
                projection::bound_record(&value, case, Some(id), Some(reference))?;
                projection::record_detail(&value, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
