use super::{projection, time::format_time};
use crate::error::ApiError;
use application::hearings::*;
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn draft(
    row: HearingDraft,
    case: CaseId,
    expected: &HearingCommand,
) -> Result<Value, ApiError> {
    if row.command != *expected
        || row
            .command
            .result_revision()
            .map_err(|_| ApiError::internal())?
            != row.result_revision
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"case_id":case,"actor_id":row.actor,"command":projection::command(&row.command)?,"result_revision":row.result_revision.get(),"values":projection::values(&row.values)?,"values_digest":row.values_digest.to_hex(),"submission_digest":row.submission_digest.to_hex()}),
    )
}
pub(super) fn page(row: HearingPage, case: CaseId) -> Result<Value, ApiError> {
    if row.hearings.iter().any(|v| v.case_id != case) {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"hearings":row.hearings.into_iter().map(projection::overview).collect::<Result<Vec<_>,_>>()?,"has_more":row.has_more,"next_after_id":row.next_after_id.map(|id|id.to_string())}),
    )
}
pub(super) fn history(
    row: HearingHistoryPage,
    case: CaseId,
    id: HearingId,
) -> Result<Value, ApiError> {
    Ok(
        json!({"revisions":row.revisions.into_iter().map(|r|projection::detail(r,case,id,None)).collect::<Result<Vec<_>,_>>()?,"has_more":row.has_more,"next_before_revision":row.next_before_revision.map(|r|r.get())}),
    )
}
pub(super) fn agenda(row: HearingAgendaPage) -> Result<Value, ApiError> {
    let next = match row.next_after {
        Some(c) => Some(json!({"at":format_time(c.at)?,"id":c.id.to_string()})),
        None => None,
    };
    Ok(
        json!({"hearings":row.hearings.into_iter().map(projection::overview).collect::<Result<Vec<_>,_>>()?,"has_more":row.has_more,"next_after":next}),
    )
}
