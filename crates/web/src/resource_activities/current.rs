use super::projection;
use crate::error::ApiError;
use application::{
    deadline_tracking::DeadlineReviewState, deadlines::DeadlineStatus, resource_activities::*,
};
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn view(
    value: ResourceActivityView,
    case: CaseId,
    resource: ResourceId,
    id: ResourceActivityId,
    revision: Option<ResourceActivityRevision>,
) -> Result<Value, ApiError> {
    let row = &value.association;
    let checked = projection::instant(value.checked_at)?;
    if row.recorded_at > value.checked_at {
        return Err(ApiError::internal());
    }
    let current = match (
        row.selection.target,
        &row.sources.target,
        value.current_target,
    ) {
        (
            ResourceActivityTarget::Hearing { id, revision, .. },
            ResourceActivityTargetDetail::Hearing(captured),
            ResourceActivityCurrentTarget::Hearing(current),
        ) => {
            if current.snapshot.revision < revision
                || current.snapshot.recorded_at > value.checked_at
                || current.snapshot.revision == revision && current.as_ref() != captured.as_ref()
            {
                return Err(ApiError::internal());
            }
            json!({"kind":"hearing","record":crate::hearings::exact_projection(*current,case,id,None)?})
        }
        (
            ResourceActivityTarget::Deadline { id, revision, .. },
            ResourceActivityTargetDetail::Deadline(captured),
            ResourceActivityCurrentTarget::Deadline(current),
        ) => {
            let detail = current.detail();
            let operational = current.operational();
            let unchecked = detail.status == DeadlineStatus::Retired
                || detail.review_state() == DeadlineReviewState::LegacyUndeclared;
            if detail.revision < revision
                || detail.recorded_at > value.checked_at
                || detail.revision == revision && detail != captured.as_ref()
                || !operational.matches_capture(detail)
                || match operational.checked_at() {
                    Some(at) => at != value.checked_at || at.offset() != time::UtcOffset::UTC,
                    None => !unchecked,
                }
            {
                return Err(ApiError::internal());
            }
            json!({"kind":"deadline","record":crate::deadlines::current_projection(*current,case,id)?})
        }
        _ => return Err(ApiError::internal()),
    };
    Ok(
        json!({"association":projection::detail(value.association,case,resource,id,revision)?,"checked_at":checked,"current_target":current}),
    )
}
