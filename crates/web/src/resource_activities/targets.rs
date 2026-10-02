use super::{case, current, projection, query, request, ResourceActivityState};
use crate::{error::ApiError, request::bearer_token};
use application::{deadlines::DeadlineId, hearings::HearingId, resource_activities::*};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};

type PageInput = Result<Query<query::TargetPage>, QueryRejection>;

pub(super) async fn hearing(
    State(state): State<ResourceActivityState>,
    Path(path): Path<(String, String)>,
    headers: HeaderMap,
    input: PageInput,
) -> Result<Json<Value>, ApiError> {
    list(state, path, headers, input, ResourceActivityKind::Hearing).await
}

pub(super) async fn deadline(
    State(state): State<ResourceActivityState>,
    Path(path): Path<(String, String)>,
    headers: HeaderMap,
    input: PageInput,
) -> Result<Json<Value>, ApiError> {
    list(state, path, headers, input, ResourceActivityKind::Deadline).await
}

async fn list(
    state: ResourceActivityState,
    (case_path, target_path): (String, String),
    headers: HeaderMap,
    input: PageInput,
    kind: ResourceActivityKind,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = case(&case_path)?;
    let id = request::uuid(&target_path)?;
    let target = match kind {
        ResourceActivityKind::Hearing => {
            ResourceActivityTargetId::Hearing(HearingId::from_uuid(id))
        }
        ResourceActivityKind::Deadline => {
            ResourceActivityTargetId::Deadline(DeadlineId::from_uuid(id))
        }
    };
    let Query(input) = input.map_err(|_| query::invalid())?;
    let query = input.validate()?;
    let page = state
        .runtime
        .run(move || state.workflow.list_for_target(&token, case, target, query))
        .await?;
    let checked_at = projection::instant(page.checked_at)?;
    if page.associations.len() > query.limit() as usize
        || page.has_more != page.next_after_id.is_some()
        || page.has_more
            && (page.associations.len() != query.limit() as usize
                || page.associations.last().map(|v| v.association.id) != page.next_after_id)
        || page.associations.iter().any(|v| {
            v.checked_at != page.checked_at
                || !matches_target(target, v.association.selection.target)
                || query
                    .after_id()
                    .is_some_and(|id| v.association.id.as_uuid() <= id.as_uuid())
                || query
                    .status()
                    .is_some_and(|status| v.association.status != status)
        })
        || page
            .associations
            .windows(2)
            .any(|v| v[0].association.id.as_uuid() >= v[1].association.id.as_uuid())
    {
        return Err(ApiError::internal());
    }
    let associations = page
        .associations
        .into_iter()
        .map(|row| {
            let resource = row.association.resource_id;
            let id = row.association.id;
            current::view(row, case, resource, id, None)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(json!({
        "case_id":case,
        "target":{"kind":kind.as_str(),"id":id.to_string()},
        "checked_at":checked_at,
        "associations":associations,
        "has_more":page.has_more,
        "next_after_id":page.next_after_id.map(|v|v.to_string())
    })))
}

fn matches_target(expected: ResourceActivityTargetId, captured: ResourceActivityTarget) -> bool {
    match (expected, captured) {
        (
            ResourceActivityTargetId::Hearing(expected),
            ResourceActivityTarget::Hearing { id, .. },
        ) => id == expected,
        (
            ResourceActivityTargetId::Deadline(expected),
            ResourceActivityTarget::Deadline { id, .. },
        ) => id == expected,
        _ => false,
    }
}
