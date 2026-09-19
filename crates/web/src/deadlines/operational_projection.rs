use application::{
    deadline_currentness::{DeadlineFreshness, DeadlineOperational},
    deadline_tracking::TrackingDependency,
};
use serde_json::{json, Value};
pub(super) fn project(value: &DeadlineOperational) -> Value {
    let freshness = match value.freshness() {
        DeadlineFreshness::NotChecked => "not_checked",
        DeadlineFreshness::Current => "current",
        DeadlineFreshness::Changed => "changed",
    };
    let changed: Vec<_> = value
        .changed_dependencies()
        .iter()
        .map(|dependency| match dependency {
            TrackingDependency::Profile => "profile",
            TrackingDependency::Source => "source",
            TrackingDependency::Calendar => "calendar",
        })
        .collect();
    json!({"freshness":freshness,"checked_at":value.checked_at().map(super::result::instant),"changed_dependencies":changed,"due_at":value.due_at().map(super::result::instant)})
}
