use super::definition_projection as p;
use crate::error::ApiError;
use application::deadline_profiles::*;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
pub(super) fn collection(c: DeadlineProfileCollection) -> Value {
    match c {
        DeadlineProfileCollection::Global => json!({"kind":"global"}),
        DeadlineProfileCollection::ForCase(id) => json!({"kind":"case","case_id":id.to_string()}),
    }
}
fn definition_matches(v: &DeadlineProfileDefinition, c: &DeadlineProfileCommand) -> bool {
    match &c.change {
        DeadlineProfileChange::Publish { definition }
        | DeadlineProfileChange::Replace { definition, .. } => {
            deadline_profile_definition_bytes(v) == deadline_profile_definition_bytes(definition)
        }
        DeadlineProfileChange::Retire { .. } => true,
    }
}
fn command(v: &DeadlineProfileCommand) -> Result<Value, ApiError> {
    let mut change =
        json!({"action":v.action().as_str(),"expected_revision":v.expected_revision()});
    if let Some(reason) = v.reason() {
        change["reason"] = json!(reason.as_str());
    }
    match &v.change {
        DeadlineProfileChange::Publish { definition }
        | DeadlineProfileChange::Replace { definition, .. } => {
            change["definition"] = p::definition(definition)?
        }
        _ => {}
    }
    Ok(
        json!({"operation_id":v.operation_id.to_string(),"profile_id":v.profile_id.to_string(),"change":change}),
    )
}
pub(super) fn draft(
    v: DeadlineProfileDraft,
    c: DeadlineProfileCollection,
    expected: &DeadlineProfileCommand,
) -> Result<Value, ApiError> {
    if v.collection != c
        || v.command != *expected
        || v.result_revision != expected.result_revision()?
        || v.initial_scope != *v.definition.scope()
        || !c.permits_mutation(&v.initial_scope)
        || !definition_matches(&v.definition, expected)
        || !definition_matches(&v.definition, &v.command)
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"collection":collection(c),"actor_id":v.actor,"command":command(&v.command)?,"result_revision":v.result_revision.get(),
        "initial_scope":p::scope(&v.initial_scope),"definition":p::definition(&v.definition)?,"algorithm":v.algorithm.as_str(),
        "definition_digest":v.definition_digest.to_hex(),"submission_digest":v.submission_digest.to_hex()}),
    )
}
pub(super) fn submitted(
    v: &DeadlineProfileDetail,
    expected: &DeadlineProfileCommand,
    digest: domain::crypto::Sha256Digest,
) -> Result<(), ApiError> {
    if v.receipt.operation_id != expected.operation_id
        || v.receipt.action != expected.action()
        || v.receipt.expected_revision != expected.expected_revision()
        || v.receipt.submission_digest != digest
        || v.reason.as_ref() != expected.reason()
        || v.status != expected.result_status()
        || !definition_matches(&v.definition, expected)
    {
        return Err(ApiError::internal());
    }
    Ok(())
}
pub(super) fn detail(
    v: DeadlineProfileDetail,
    c: DeadlineProfileCollection,
    id: DeadlineProfileId,
    revision: Option<DeadlineProfileRevision>,
) -> Result<Value, ApiError> {
    if revision.is_some_and(|r| r != v.revision) {
        return Err(ApiError::internal());
    }
    let mut row = entry(DeadlineProfileHistoryEntry::from(&v), c, id)?;
    row["definition"] = p::definition(&v.definition)?;
    row["collection"] = collection(c);
    Ok(row)
}
fn entry(
    v: DeadlineProfileHistoryEntry,
    c: DeadlineProfileCollection,
    id: DeadlineProfileId,
) -> Result<Value, ApiError> {
    let shape = match v.receipt.action {
        DeadlineProfileAction::Publish => {
            v.receipt.expected_revision == 0
                && v.reason.is_none()
                && v.status == DeadlineProfileStatus::Published
        }
        DeadlineProfileAction::Replace => {
            v.receipt.expected_revision > 0
                && v.reason.is_some()
                && v.status == DeadlineProfileStatus::Published
        }
        DeadlineProfileAction::Retire => {
            v.receipt.expected_revision > 0
                && v.reason.is_some()
                && v.status == DeadlineProfileStatus::Retired
        }
    };
    if v.id != id
        || !c.includes(&v.scope)
        || !shape
        || v.receipt.expected_revision.checked_add(1) != Some(v.revision.get())
        || v.recorded_by.email.trim().is_empty()
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"id":v.id.to_string(),"revision":v.revision.get(),"status":v.status.as_str(),"algorithm":v.algorithm.as_str(),"definition_digest":v.definition_digest.to_hex(),
        "scope":p::scope(&v.scope),"reason":v.reason.as_ref().map(|v|v.as_str()),"receipt":{"operation_id":v.receipt.operation_id.to_string(),"action":v.receipt.action.as_str(),
        "expected_revision":v.receipt.expected_revision,"submission_digest":v.receipt.submission_digest.to_hex()},
        "recorded_at":v.recorded_at.format(&Rfc3339).map_err(|_|ApiError::internal())?,"recorded_by":{"id":v.recorded_by.id,"email":v.recorded_by.email}}),
    )
}
pub(super) fn page(
    v: DeadlineProfilePage,
    c: DeadlineProfileCollection,
    q: &DeadlineProfileQuery,
) -> Result<Value, ApiError> {
    if v.profiles.len() > q.limit() as usize
        || v.has_more && v.profiles.len() != q.limit() as usize
        || v.next_after_id
            != if v.has_more {
                v.profiles.last().map(|v| v.id)
            } else {
                None
            }
    {
        return Err(ApiError::internal());
    }
    let mut after = q.after_id();
    for row in &v.profiles {
        if !c.includes(&row.scope)
            || after.is_some_and(|id| row.id.as_uuid() <= id.as_uuid())
            || q.status().status().is_some_and(|s| row.status != s)
        {
            return Err(ApiError::internal());
        }
        after = Some(row.id);
    }
    Ok(
        json!({"collection":collection(c),"profiles":v.profiles.iter().map(|v|json!({"id":v.id.to_string(),"revision":v.revision.get(),"status":v.status.as_str(),"algorithm":v.algorithm.as_str(),
        "definition_digest":v.definition_digest.to_hex(),"title":v.title.as_str(),"scope":p::scope(&v.scope)})).collect::<Vec<_>>(),"has_more":v.has_more,"next_after_id":v.next_after_id.map(|v|v.to_string())}),
    )
}
pub(super) fn history(
    v: DeadlineProfileHistoryPage,
    c: DeadlineProfileCollection,
    id: DeadlineProfileId,
    q: DeadlineProfileHistoryQuery,
) -> Result<Value, ApiError> {
    if v.revisions.len() > q.limit() as usize
        || v.has_more && v.revisions.len() != q.limit() as usize
        || v.next_before_revision
            != if v.has_more {
                v.revisions.last().map(|v| v.revision)
            } else {
                None
            }
        || v.revisions.iter().any(|v| {
            q.before_revision()
                .is_some_and(|before| v.revision >= before)
        })
    {
        return Err(ApiError::internal());
    }
    for pair in v.revisions.windows(2) {
        if pair[1].revision.get().checked_add(1) != Some(pair[0].revision.get())
            || pair[0].scope != pair[1].scope
            || pair[1].status == DeadlineProfileStatus::Retired
            || pair[0].status == DeadlineProfileStatus::Retired
                && (pair[0].definition_digest != pair[1].definition_digest
                    || pair[0].algorithm != pair[1].algorithm)
        {
            return Err(ApiError::internal());
        }
    }
    Ok(
        json!({"collection":collection(c),"revisions":v.revisions.into_iter().map(|v|entry(v,c,id)).collect::<Result<Vec<_>,_>>()?,"has_more":v.has_more,"next_before_revision":v.next_before_revision.map(|v|v.get())}),
    )
}
