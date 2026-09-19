//! Decisions for changes to explicitly tracked deadline dependencies.
//!
//! The persistence port verifies dependency identity, case scope, the actual
//! revisions and their retirement status before invoking this policy. Revision
//! numbers alone do not establish those facts. These decisions neither reserve
//! a deadline revision nor accept a new human qualification.

mod review;
pub use review::{
    TrackingPolicies, TrackingReview, TrackingReviewError, TrackingReviewRequirement,
};

use domain::deadlines::DeadlineStatus;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingDependency {
    Profile,
    Source,
    Calendar,
}

/// A policy is declared separately for each dependency; equality of selected
/// and observed revisions never supplies a missing declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingPolicy {
    Follow,
    Fixed,
    Undetermined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackingChange {
    pub dependency: TrackingDependency,
    pub policy: TrackingPolicy,
    pub selected_revision: u32,
    pub observed_revision: u32,
    pub new_revision: u32,
    /// Whether the incoming revision retires or withdraws the dependency.
    pub retired: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingReviewReason {
    SourceChanged,
    ProfileChanged,
    DependencyRetired,
    PolicyUndetermined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingDisposition {
    /// The event was already observed. Existing review requirements remain.
    Unchanged,
    /// Observe the new head while retaining the explicitly fixed selection.
    Preserve,
    /// Calculate with the proposed selection; this does not activate a due date.
    Recalculate,
    ReviewRequired(TrackingReviewReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackingDecision {
    pub selected_revision: u32,
    pub observed_revision: u32,
    pub disposition: TrackingDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TrackingError {
    #[error("tracking revisions must be positive")]
    InvalidRevision,
    #[error("the selected revision is newer than its observed head")]
    InconsistentRevisions,
}

/// Decide a dependency change without replacing retained historical evidence.
///
/// The caller verifies the dependency's exact identity, scope and persisted
/// revisions. Old or repeated events do not reopen a completed review. A newly
/// observed retirement requires review even for a fixed selection. Following
/// a source or profile never transfers a human qualification automatically.
/// No database write, revision reservation or calculation occurs here.
pub fn decide_tracking_change(change: TrackingChange) -> Result<TrackingDecision, TrackingError> {
    if change.selected_revision == 0 || change.observed_revision == 0 || change.new_revision == 0 {
        return Err(TrackingError::InvalidRevision);
    }
    if change.selected_revision > change.observed_revision {
        return Err(TrackingError::InconsistentRevisions);
    }
    let mut decision = TrackingDecision {
        selected_revision: change.selected_revision,
        observed_revision: change.observed_revision,
        disposition: TrackingDisposition::Unchanged,
    };
    if change.new_revision <= change.observed_revision {
        return Ok(decision);
    }
    decision.observed_revision = change.new_revision;
    decision.disposition = if change.retired {
        TrackingDisposition::ReviewRequired(TrackingReviewReason::DependencyRetired)
    } else {
        match (change.policy, change.dependency) {
            (TrackingPolicy::Fixed, _) => TrackingDisposition::Preserve,
            (TrackingPolicy::Undetermined, _) => {
                TrackingDisposition::ReviewRequired(TrackingReviewReason::PolicyUndetermined)
            }
            (TrackingPolicy::Follow, TrackingDependency::Calendar) => {
                decision.selected_revision = change.new_revision;
                TrackingDisposition::Recalculate
            }
            (TrackingPolicy::Follow, TrackingDependency::Source) => {
                TrackingDisposition::ReviewRequired(TrackingReviewReason::SourceChanged)
            }
            (TrackingPolicy::Follow, TrackingDependency::Profile) => {
                TrackingDisposition::ReviewRequired(TrackingReviewReason::ProfileChanged)
            }
        }
    };
    Ok(decision)
}

/// Acceptance applies to the current captured inputs. Historical captures
/// without an explicit review state do not imply operational acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineReviewState {
    Accepted,
    Pending,
    LegacyUndeclared,
}

/// Project a captured due only when the registry and current review permit it.
/// This never changes historical evidence, fills a missing instant or decides
/// whether the supplied acceptance was authorized; the caller verifies that.
pub fn operational_due_at(
    captured_due: Option<OffsetDateTime>,
    status: DeadlineStatus,
    review: DeadlineReviewState,
) -> Option<OffsetDateTime> {
    match (status, review) {
        (DeadlineStatus::Active, DeadlineReviewState::Accepted) => captured_due,
        _ => None,
    }
}
