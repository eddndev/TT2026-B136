use super::{context, request, sources, values};
use crate::error::ApiError;
use application::procedural_resources::*;
use domain::cases::CaseId;
use serde_json::{json, Value};
pub(super) fn previous(v: Option<ResourceRevisionRef>) -> Value {
    json!(v.map(|v| json!({"revision":v.revision.get(),"capture_digest":v.capture_digest.to_hex()})))
}
fn check_previous(
    v: Option<ResourceRevisionRef>,
    revision: ResourceRevision,
) -> Result<(), ApiError> {
    match (v, revision.get()) {
        (None, 1) => Ok(()),
        (Some(p), n) if p.revision.get() == n - 1 => Ok(()),
        _ => Err(ApiError::internal()),
    }
}
fn check_act(
    v: Option<&ResourceActCapture>,
    action: ResourceAction,
    revision: ResourceRevision,
) -> Result<(), ApiError> {
    match (v, action) {
        (Some(v), ResourceAction::RecordAct) if v.revision.get() == 1 && v.previous.is_none() => {}
        (Some(v), ResourceAction::CorrectAct)
            if v.revision.get() > 1 && v.previous.is_some_and(|p| p.revision < revision) => {}
        (
            None,
            ResourceAction::Register
            | ResourceAction::Correct
            | ResourceAction::Archive
            | ResourceAction::Reactivate,
        ) => {}
        _ => return Err(ApiError::internal()),
    }
    if let Some(v) = v {
        sources::validate_supports(&v.values.direct_supports(), &v.supports)?;
    }
    Ok(())
}
fn act(v: &ResourceActCapture) -> Result<Value, ApiError> {
    Ok(
        json!({"id":v.id.to_string(),"revision":v.revision.get(),"values":values::act(&v.values)?,
        "supports":sources::supports(&v.supports)?,"previous":previous(v.previous)}),
    )
}
pub(crate) fn detail(
    row: ResourceDetail,
    case: CaseId,
    id: ResourceId,
    revision: Option<ResourceRevision>,
) -> Result<Value, ApiError> {
    if row.case_id != case
        || row.id != id
        || revision.is_some_and(|r| r != row.revision)
        || row.receipt.expected_revision.checked_add(1) != Some(row.revision.get())
        || (row.receipt.action == ResourceAction::Register) != (row.revision.get() == 1)
        || matches!(
            row.receipt.action,
            ResourceAction::Correct
                | ResourceAction::CorrectAct
                | ResourceAction::Archive
                | ResourceAction::Reactivate
        ) != row.reason.is_some()
        || (row.receipt.action == ResourceAction::Archive)
            != (row.status == ResourceStatus::Archived)
    {
        return Err(ApiError::internal());
    }
    check_previous(row.receipt.previous, row.revision)?;
    check_act(row.act.as_ref(), row.receipt.action, row.revision)?;
    sources::validate(case, &row.values, &row.sources)?;
    let r = &row.receipt;
    Ok(
        json!({"case_id":case,"id":id.to_string(),"revision":row.revision.get(),
        "values":values::values(&row.values)?,"status":row.status.as_str(),"sources":sources::sources(&row.sources)?,
        "act":row.act.as_ref().map(act).transpose()?,"reason":row.reason.as_ref().map(|r|r.as_str()),
        "receipt":{"operation_id":r.operation_id.to_string(),"action":request::action(r.action),
            "expected_revision":r.expected_revision,"previous":previous(r.previous),
            "values_digest":r.values_digest.to_hex(),"sources_digest":r.sources_digest.to_hex(),
            "submission_digest":r.submission_digest.to_hex(),"capture_digest":r.capture_digest.to_hex()},
        "recorded_by":context::actor(&row.recorded_by)?,"recorded_at":context::utc(row.recorded_at)?,
        "recorded_administration":context::administration(&row.recorded_administration,case)?,
        "recorded_stage":context::stage(&row.recorded_stage,&row.recorded_administration,case)?}),
    )
}
pub(super) fn draft(
    row: ResourceDraft,
    case: CaseId,
    command: &ResourceCommand,
) -> Result<Value, ApiError> {
    if row.case_id != case
        || row.command != *command
        || row.result_revision != command.result_revision()?
        || (command.action() == ResourceAction::Archive) != (row.status == ResourceStatus::Archived)
    {
        return Err(ApiError::internal());
    }
    check_previous(row.previous, row.result_revision)?;
    check_act(row.act.as_ref(), command.action(), row.result_revision)?;
    match (&command.change, &row.act) {
        (ResourceChange::Register { values } | ResourceChange::Correct { values, .. }, None)
            if *values == row.values => {}
        (ResourceChange::RecordAct { act_id, values, .. }, Some(act))
            if *act_id == act.id && *values == act.values => {}
        (
            ResourceChange::CorrectAct {
                act_id,
                expected_act_revision,
                values,
                ..
            },
            Some(act),
        ) if *act_id == act.id
            && expected_act_revision.get().checked_add(1) == Some(act.revision.get())
            && *values == act.values => {}
        (ResourceChange::Archive { .. } | ResourceChange::Reactivate { .. }, None) => {}
        _ => return Err(ApiError::internal()),
    }
    sources::validate(case, &row.values, &row.sources)?;
    Ok(
        json!({"case_id":case,"command":request::project(command)?,"result_revision":row.result_revision.get(),
        "values":values::values(&row.values)?,"status":row.status.as_str(),"sources":sources::sources(&row.sources)?,
        "act":row.act.as_ref().map(act).transpose()?,"previous":previous(row.previous),
        "recorded_by":context::actor(&row.recorded_by)?,
        "observed_administration":context::administration(&row.observed_administration,case)?,
        "observed_stage":context::stage(&row.observed_stage,&row.observed_administration,case)?,
        "submission_digest":row.submission_digest.to_hex()}),
    )
}
