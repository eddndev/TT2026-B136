use super::{sources, values};
use crate::error::ApiError;
use application::procedural_facts::*;
use domain::cases::CaseId;
use serde_json::{json, Value};
#[path = "response_metadata.rs"]
mod metadata;

pub(super) fn detail(
    row: FactDetail,
    case: CaseId,
    expected: FactTarget,
    revision: Option<FactRevision>,
) -> Result<Value, ApiError> {
    let snapshot = &row.snapshot;
    if snapshot.case_id() != case
        || snapshot.target() != expected
        || revision.is_some_and(|r| r != snapshot.metadata().revision)
    {
        return Err(ApiError::internal());
    }
    let declared = snapshot.values();
    validate_values_target(&declared, expected)?;
    sources::validate(case, &declared, &row.sources)?;
    let mut value = metadata::metadata(snapshot.metadata(), case)?;
    merge(&mut value, target(case, expected));
    value["values"] = project_values(&declared)?;
    value["sources"] = sources::project(&row.sources)?;
    Ok(value)
}

pub(super) fn history_entry(
    entry: &FactHistoryEntry,
    case: CaseId,
    expected: FactTarget,
) -> Result<Value, ApiError> {
    if entry.case_id != case || entry.target != expected {
        return Err(ApiError::internal());
    }
    let mut value = metadata::metadata(&entry.metadata, case)?;
    merge(&mut value, target(case, expected));
    Ok(value)
}

pub(super) fn project_values(value: &ProceduralFactValues) -> Result<Value, ApiError> {
    match value {
        ProceduralFactValues::Resolution(v) => values::resolution(v),
        ProceduralFactValues::Notification(v) => values::notification(v),
    }
}

pub(super) fn validate_values_target(
    values: &ProceduralFactValues,
    target: FactTarget,
) -> Result<(), ApiError> {
    match (values, target) {
        (ProceduralFactValues::Resolution(_), FactTarget::Resolution(_)) => Ok(()),
        (ProceduralFactValues::Notification(v), FactTarget::Notification { resolution_id, .. })
            if v.resolution().id == resolution_id =>
        {
            Ok(())
        }
        _ => Err(ApiError::internal()),
    }
}

pub(super) fn target(case: CaseId, target: FactTarget) -> Value {
    match target {
        FactTarget::Resolution(id) => {
            json!({"case_id":case,"family":"resolution","id":id.to_string()})
        }
        FactTarget::Notification { id, resolution_id } => json!({
            "case_id":case,"family":"notification","id":id.to_string(),"resolution_id":resolution_id.to_string()
        }),
    }
}
pub(super) fn administration(
    current: &application::cases::CurrentCaseAdministration,
    case: CaseId,
) -> Result<Value, ApiError> {
    metadata::administration(current, case)
}
pub(super) fn status(value: FactStatus) -> &'static str {
    metadata::status(value)
}
pub(super) fn merge(target: &mut Value, source: Value) {
    if let (Some(target), Value::Object(source)) = (target.as_object_mut(), source) {
        target.extend(source);
    }
}
