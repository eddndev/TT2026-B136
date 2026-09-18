use super::{metadata, result};
use crate::error::ApiError;
use application::deadlines::*;
use domain::cases::CaseId;
use serde_json::{json, Value};
use std::collections::HashSet;
pub(super) fn page(v: DeadlinePage, case: CaseId, q: &DeadlineQuery) -> Result<Value, ApiError> {
    if v.deadlines.len() > q.limit() as usize
        || v.has_more && v.deadlines.len() != q.limit() as usize
        || v.next_after_id
            != if v.has_more {
                v.deadlines.last().map(|v| v.id)
            } else {
                None
            }
    {
        return Err(ApiError::internal());
    }
    let mut after = q.after_id();
    let mut rows = Vec::with_capacity(v.deadlines.len());
    for v in v.deadlines {
        if v.case_id != case
            || after.is_some_and(|id| v.id.as_uuid() <= id.as_uuid())
            || q.status().status().is_some_and(|s| s != v.status)
            || v.blocked != v.due_at.is_none()
        {
            return Err(ApiError::internal());
        }
        after = Some(v.id);
        rows.push(json!({"id":v.id.to_string(),"case_id":case.to_string(),"revision":v.revision.get(),"title":v.title.as_str(),"status":v.status.as_str(),"responsible":metadata::responsible(&v.responsible)?,"attention_recorded":v.attention_recorded,"due_at":v.due_at.map(result::instant),"blocked":v.blocked}));
    }
    Ok(
        json!({"case_id":case.to_string(),"deadlines":rows,"has_more":v.has_more,"next_after_id":v.next_after_id.map(|id|id.to_string())}),
    )
}
pub(super) fn history(
    v: DeadlineHistoryPage,
    case: CaseId,
    id: DeadlineId,
    q: DeadlineHistoryQuery,
) -> Result<Value, ApiError> {
    if v.revisions.len() > q.limit() as usize
        || v.has_more && v.revisions.len() != q.limit() as usize
        || v.next_before_revision
            != if v.has_more {
                v.revisions.last().map(|v| v.revision)
            } else {
                None
            }
        || v.revisions.is_empty() && q.before_revision().map(|r| r.get()) != Some(1)
        || v.has_more && v.revisions.last().is_some_and(|r| r.revision.get() == 1)
        || !v.has_more && v.revisions.last().is_some_and(|r| r.revision.get() != 1)
    {
        return Err(ApiError::internal());
    }
    for pair in v.revisions.windows(2) {
        if pair[1].revision.get().checked_add(1) != Some(pair[0].revision.get())
            || pair[1].status == DeadlineStatus::Retired
        {
            return Err(ApiError::internal());
        }
    }
    let mut operations = HashSet::new();
    let mut rows = Vec::with_capacity(v.revisions.len());
    for v in v.revisions {
        if v.case_id != case
            || v.id != id
            || q.before_revision().is_some_and(|r| v.revision >= r)
            || v.recorded_by.email.trim().is_empty()
            || v.state_digest != v.receipt.review_digest
            || !operations.insert(v.receipt.operation_id)
        {
            return Err(ApiError::internal());
        }
        metadata::shape(&v.receipt, v.revision, v.status, v.reason.as_ref())?;
        rows.push(json!({"id":id.to_string(),"case_id":case.to_string(),"revision":v.revision.get(),"status":v.status.as_str(),"reason":v.reason.as_ref().map(|r|r.as_str()),"receipt":metadata::receipt(&v.receipt),"state_digest":v.state_digest.to_hex(),"recorded_at":result::instant(v.recorded_at),"recorded_by":{"id":v.recorded_by.id,"email":v.recorded_by.email}}));
    }
    Ok(
        json!({"case_id":case.to_string(),"id":id.to_string(),"revisions":rows,"has_more":v.has_more,"next_before_revision":v.next_before_revision.map(|r|r.get())}),
    )
}
