use super::{projection, sources};
use crate::error::ApiError;
use application::hearing_results::*;
use domain::{cases::CaseId, hearings::HearingId};
use serde_json::{json, Value};
pub(super) fn draft(
    row: HearingResultDraft,
    case: CaseId,
    expected: &HearingResultCommand,
) -> Result<Value, ApiError> {
    if row.case_id != case
        || row.command != *expected
        || row.result_revision != row.command.result_revision()?
    {
        return Err(ApiError::internal());
    }
    sources::validate(
        case,
        expected.hearing_id,
        expected.result_id,
        &row.values,
        &row.anchor,
        row.continuation.as_ref(),
        &row.attendees,
        row.support.as_ref(),
    )?;
    let admin = row
        .observed_administration
        .snapshot()
        .ok_or_else(ApiError::internal)?;
    if admin.case_id != case {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"case_id":case,"actor_id":row.actor,"command":projection::command(&row.command)?,"result_revision":row.result_revision.get(),"values":projection::values(&row.values)?,"values_digest":row.values_digest.to_hex(),"submission_digest":row.submission_digest.to_hex(),
        "anchor":sources::anchor(&row.anchor)?,"continuation":row.continuation.as_ref().map(sources::continuation),"observed_administration":{"revision":admin.revision.get(),"values_digest":admin.values_digest.to_hex(),"status":admin.values.status().as_str()},"attendees":sources::attendees(&row.attendees,&row.values),"support":row.support.as_ref().map(sources::support)}),
    )
}
pub(super) fn page(
    row: HearingResultPage,
    case: CaseId,
    hearing: HearingId,
) -> Result<Value, ApiError> {
    if row
        .results
        .iter()
        .any(|r| r.case_id != case || r.hearing_id != hearing)
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"results":row.results.into_iter().map(projection::overview).collect::<Result<Vec<_>,_>>()?,"has_more":row.has_more,"next_after_id":row.next_after_id.map(|id|id.to_string())}),
    )
}
pub(super) fn history(
    row: HearingResultHistoryPage,
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
) -> Result<Value, ApiError> {
    Ok(
        json!({"revisions":row.revisions.into_iter().map(|r|projection::history_entry(r,case,hearing,id)).collect::<Result<Vec<_>,_>>()?,"has_more":row.has_more,"next_before_revision":row.next_before_revision.map(|r|r.get())}),
    )
}
