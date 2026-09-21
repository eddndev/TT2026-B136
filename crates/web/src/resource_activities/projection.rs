use super::{selection, sources};
use crate::{error::ApiError, procedural_resources as resources};
use application::resource_activities::*;
use domain::cases::CaseId;
use serde_json::{json, Value};
use time::{OffsetDateTime, UtcOffset};

pub(super) fn previous(v: Option<ResourceActivityRevisionRef>) -> Value {
    json!(v.map(|p| json!({"revision":p.revision.get(),"capture_digest":p.capture_digest.to_hex()})))
}
fn context(
    selection: ResourceActivitySelection,
    head: ResourceCaptureRef,
    resource: ResourceId,
    revision: ResourceActivityRevision,
    prior: Option<ResourceActivityRevisionRef>,
) -> Result<(), ApiError> {
    if head.id != resource
        || selection.resource.id != resource
        || selection.resource.revision > head.revision
        || selection
            .act
            .is_some_and(|a| a.resource_revision > head.revision)
        || selection.resource.revision == head.revision
            && selection.resource.capture_digest != head.capture_digest
        || selection.act.is_some_and(|a| {
            a.resource_revision == head.revision && a.capture_digest != head.capture_digest
        })
    {
        return Err(ApiError::internal());
    }
    match (prior, revision.get()) {
        (None, 1) => Ok(()),
        (Some(p), n) if p.revision.get() == n - 1 => Ok(()),
        _ => Err(ApiError::internal()),
    }
}
pub(super) fn instant(v: OffsetDateTime) -> Result<Value, ApiError> {
    if v.offset() != UtcOffset::UTC || !(1..=9999).contains(&v.year()) {
        return Err(ApiError::internal());
    }
    Ok(json!({"unix_seconds":v.unix_timestamp(),"nanosecond":v.nanosecond(),"offset_seconds":0}))
}
pub(super) fn detail(
    row: ResourceActivityDetail,
    case: CaseId,
    resource: ResourceId,
    id: ResourceActivityId,
    revision: Option<ResourceActivityRevision>,
) -> Result<Value, ApiError> {
    let command = resource_activity_command_from_detail(&row).map_err(|_| ApiError::internal())?;
    if row.case_id != case
        || row.resource_id != resource
        || row.id != id
        || revision.is_some_and(|v| v != row.revision)
        || command
            .result_revision()
            .map_err(|_| ApiError::internal())?
            != row.revision
        || row.status != status(command.action())
        || row.recorded_resource_head.revision != command.expected_resource_revision
        || row.recorded_at < sources::latest_time(&row.sources)
        || row
            .recorded_administration
            .snapshot()
            .is_some_and(|a| a.changed_at > row.recorded_at)
    {
        return Err(ApiError::internal());
    }
    context(
        row.selection,
        row.recorded_resource_head,
        resource,
        row.revision,
        row.receipt.previous,
    )?;
    instant(row.recorded_at)?;
    let r = row.receipt;
    Ok(
        json!({"case_id":case,"resource_id":resource.to_string(),"id":id.to_string(),"revision":row.revision.get(),
        "selection":selection::project(row.selection),"status":row.status.as_str(),"reason":row.reason.as_ref().map(|r|r.as_str()),
        "sources":sources::project(row.sources,case,resource,row.selection)?,
        "receipt":{"operation_id":r.operation_id.to_string(),"action":r.action.as_str(),"expected_revision":r.expected_revision,
            "expected_resource_revision":r.expected_resource_revision.get(),"previous":previous(r.previous),"submission_digest":r.submission_digest.to_hex(),"capture_digest":r.capture_digest.to_hex()},
        "recorded_by":resources::project_actor(&row.recorded_by)?,"recorded_at":resources::utc(row.recorded_at)?,
        "recorded_administration":resources::project_administration(&row.recorded_administration,case)?,
        "recorded_resource_head":selection::resource(row.recorded_resource_head)}),
    )
}
pub(super) fn draft(
    row: ResourceActivityDraft,
    case: CaseId,
    resource: ResourceId,
    expected: &ResourceActivityCommand,
) -> Result<Value, ApiError> {
    if row.case_id != case
        || row.resource_id != resource
        || row.command != *expected
        || row.result_revision
            != expected
                .result_revision()
                .map_err(|_| ApiError::internal())?
        || row.status != status(expected.action())
        || row.observed_resource_head.revision != expected.expected_resource_revision
        || matches!(&expected.change, ResourceActivityChange::Link{selection} if *selection != row.selection)
    {
        return Err(ApiError::internal());
    }
    context(
        row.selection,
        row.observed_resource_head,
        resource,
        row.result_revision,
        row.previous,
    )?;
    Ok(
        json!({"case_id":case,"resource_id":resource.to_string(),"command":selection::command(expected,case,resource),
        "result_revision":row.result_revision.get(),"selection":selection::project(row.selection),"status":row.status.as_str(),
        "sources":sources::project(row.sources,case,resource,row.selection)?,"previous":previous(row.previous),
        "recorded_by":resources::project_actor(&row.recorded_by)?,"observed_administration":resources::project_administration(&row.observed_administration,case)?,
        "observed_resource_head":selection::resource(row.observed_resource_head),"submission_digest":row.submission_digest.to_hex()}),
    )
}
fn status(action: ResourceActivityAction) -> ResourceActivityStatus {
    match action {
        ResourceActivityAction::Link => ResourceActivityStatus::Linked,
        ResourceActivityAction::Unlink => ResourceActivityStatus::Unlinked,
    }
}
