use crate::{error::ApiError, procedural_resources as resources};
use application::resource_activities::*;
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn project(
    values: ResourceActivitySources,
    case: CaseId,
    resource: ResourceId,
    selection: ResourceActivitySelection,
) -> Result<Value, ApiError> {
    if selection.resource.id != resource
        || values.resource.receipt.capture_digest != selection.resource.capture_digest
    {
        return Err(ApiError::internal());
    }
    let source = resources::exact_projection(
        values.resource,
        case,
        resource,
        Some(selection.resource.revision),
    )?;
    let act = match (selection.act, values.act) {
        (None, None) => None,
        (Some(reference), Some(row)) => {
            if row.receipt.capture_digest != reference.capture_digest
                || row
                    .act
                    .as_ref()
                    .is_none_or(|act| act.id != reference.id || act.revision != reference.revision)
            {
                return Err(ApiError::internal());
            }
            Some(resources::exact_projection(
                row,
                case,
                resource,
                Some(reference.resource_revision),
            )?)
        }
        _ => return Err(ApiError::internal()),
    };
    let target = match (selection.target, values.target) {
        (
            ResourceActivityTarget::Hearing {
                id,
                revision,
                submission_digest,
            },
            ResourceActivityTargetDetail::Hearing(row),
        ) => {
            if row.snapshot.receipt.submission_digest != submission_digest {
                return Err(ApiError::internal());
            }
            json!({"kind":"hearing","record":crate::hearings::exact_projection(*row,case,id,Some(revision))?})
        }
        (
            ResourceActivityTarget::Deadline {
                id,
                revision,
                capture_digest,
            },
            ResourceActivityTargetDetail::Deadline(row),
        ) => {
            if row.receipt.capture_digest != capture_digest {
                return Err(ApiError::internal());
            }
            json!({"kind":"deadline","record":crate::deadlines::exact_projection(*row,case,id,Some(revision))?})
        }
        _ => return Err(ApiError::internal()),
    };
    Ok(json!({"resource":source,"act":act,"target":target}))
}
pub(super) fn latest_time(values: &ResourceActivitySources) -> time::OffsetDateTime {
    let target = match &values.target {
        ResourceActivityTargetDetail::Hearing(v) => v.snapshot.recorded_at,
        ResourceActivityTargetDetail::Deadline(v) => v.recorded_at,
    };
    values
        .resource
        .recorded_at
        .max(
            values
                .act
                .as_ref()
                .map_or(values.resource.recorded_at, |a| a.recorded_at),
        )
        .max(target)
}
