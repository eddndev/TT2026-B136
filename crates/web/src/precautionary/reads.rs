use super::{primitives::*, projection, HearingState};
use crate::{error::ApiError, request::bearer_token};
use application::precautionary_hearings::*;
use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::HeaderMap,
    Json,
};
use domain::{cases::CaseId, precautionary_hearings::*};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<String>,
    after_id: Option<String>,
}
impl Page {
    fn validate(self) -> Result<PrecautionaryHearingReadQuery, ApiError> {
        let limit = self.limit.map(|v| number(&v)).transpose()?.unwrap_or(10);
        PrecautionaryHearingReadQuery::new(
            u16::try_from(limit).map_err(|_| invalid())?,
            self.after_id
                .map(|v| uuid(&v).map(PrecautionaryHearingId::from_uuid))
                .transpose()?,
        )
        .map_err(|_| invalid())
    }
}
pub(super) fn bound(
    value: &PrecautionaryHearingRecordStoredOperation,
    case: CaseId,
    hearing: Option<PrecautionaryHearingId>,
    revision: Option<PrecautionaryHearingRevision>,
    operation: Option<PrecautionaryHearingOperationId>,
) -> Result<(), ApiError> {
    let review = &value.capture.review;
    let origin = &value.history.origin;
    if review.case_id != case
        || origin.case_id != case
        || origin.hearing_id != review.command.hearing_id
        || hearing.is_some_and(|id| id != review.command.hearing_id)
        || revision.is_some_and(|r| r != review.result_revision)
        || operation.is_some_and(|id| id != review.command.operation_id)
        || value.history.captures.last() != Some(&value.capture)
        || value.history.captures.len() > 256
        || value.history.captures.len() != review.result_revision.get() as usize
        || value.history.captures.iter().any(|entry| {
            entry.review.case_id != case
                || entry.review.command.hearing_id != review.command.hearing_id
        })
    {
        return Err(ApiError::internal());
    }
    Ok(())
}
pub(super) async fn list(
    State(s): State<HearingState>,
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
                let mut items = Vec::with_capacity(page.items.len());
                for item in &page.items {
                    let id = item.capture.review.command.hearing_id;
                    if last.is_some_and(|prior| prior.as_uuid() >= id.as_uuid()) {
                        return Err(ApiError::internal());
                    }
                    bound(item, case, Some(id), None, None)?;
                    last = Some(id);
                    items.push(projection::operation(item, s.hasher.as_ref())?);
                }
                let expected = if page.has_more { last } else { None };
                if page.next_after_id != expected {
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
pub(super) async fn detail(
    State(s): State<HearingState>,
    Path((case, hearing)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    get(s, case, hearing, None, headers, q).await
}
pub(super) async fn exact(
    State(s): State<HearingState>,
    Path((case, hearing, revision)): Path<(String, String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let revision = PrecautionaryHearingRevision::new(number(&revision)?).map_err(|_| invalid())?;
    get(s, case, hearing, Some(revision), headers, q).await
}
async fn get(
    s: HearingState,
    case: String,
    hearing: String,
    revision: Option<PrecautionaryHearingRevision>,
    headers: HeaderMap,
    q: Option<String>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let hearing = PrecautionaryHearingId::from_uuid(uuid(&hearing)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let value = s.reads.get(&token, case, hearing, revision)?;
                bound(&value, case, Some(hearing), revision, None)?;
                projection::operation(&value, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
pub(super) async fn operation(
    State(s): State<HearingState>,
    Path((case, operation)): Path<(String, String)>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    empty(q.as_deref())?;
    let case = CaseId::from_uuid(uuid(&case)?);
    let operation = PrecautionaryHearingOperationId::from_uuid(uuid(&operation)?);
    let value = s
        .runtime
        .run(move || {
            Ok((|| {
                let value = s.reads.get_operation(&token, case, operation)?;
                bound(&value, case, None, None, Some(operation))?;
                projection::operation(&value, s.hasher.as_ref())
            })())
        })
        .await??;
    Ok(Json(value))
}
