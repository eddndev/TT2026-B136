use super::{evidence, inconsistent, DeadlineError};
use crate::{
    deadline_profiles::{DeadlineProfileDetail, DeadlineProfileStatus},
    deadline_tracking::TrackingPolicy,
    ApplicationError,
};

pub(super) fn validate(
    selected: &DeadlineProfileDetail,
    head: &DeadlineProfileDetail,
    policy: Option<TrackingPolicy>,
) -> Result<(), ApplicationError> {
    if selected.id != head.id
        || selected.revision > head.revision
        || selected.definition.scope() != head.definition.scope()
    {
        return Err(inconsistent(
            "selected profile and head identity or scope differ",
        ));
    }
    if selected.revision == head.revision {
        let mut selected_bytes = Vec::new();
        let mut head_bytes = Vec::new();
        evidence::profile(&mut selected_bytes, selected);
        evidence::profile(&mut head_bytes, head);
        if selected != head || selected_bytes != head_bytes {
            return Err(inconsistent(
                "same profile revision has different captured evidence",
            ));
        }
    }
    if selected.status != DeadlineProfileStatus::Published
        || head.status != DeadlineProfileStatus::Published
        || (policy != Some(TrackingPolicy::Fixed) && selected.revision != head.revision)
    {
        return Err(DeadlineError::ProfileUnavailable.into());
    }
    Ok(())
}
