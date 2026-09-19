//! Structural review requirements and their consistency with tracking policies.
//!
//! Construction does not inspect source heads or infer retirement. The caller
//! supplies every required reason; a retirement decision does not replace the
//! additional reason needed for a present dependency with an undeclared policy.

use super::{DeadlineReviewState, TrackingDependency, TrackingPolicy, TrackingReviewReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackingPolicies {
    pub profile: TrackingPolicy,
    pub source: TrackingPolicy,
    pub calendar: TrackingPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackingReviewRequirement {
    pub dependency: TrackingDependency,
    pub reason: TrackingReviewReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackingReview {
    state: DeadlineReviewState,
    reasons: Vec<TrackingReviewRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TrackingReviewError {
    #[error("tracking review has more than eight reasons")]
    TooManyReasons,
    #[error("tracking review state and reason presence disagree")]
    InvalidStateReasons,
    #[error("tracking review reason does not belong to its dependency")]
    InvalidDependencyReason,
    #[error("tracking review reasons must be unique and in canonical order")]
    NonCanonicalReasons,
    #[error("legacy tracking requires three undeclared policies")]
    LegacyPolicyDeclared,
    #[error("an absent dependency cannot have a declared policy or review reason")]
    AbsentDependency,
    #[error("an accepted present dependency requires an explicit tracking policy")]
    UndeclaredAcceptedDependency,
    #[error("a pending undeclared dependency requires its policy review reason")]
    MissingPolicyReason,
    #[error("tracking review reason disagrees with the dependency policy")]
    InvalidPolicyReason,
}

impl TrackingReview {
    /// Validate supplied reasons without sorting, deduplicating or inventing them.
    /// Dependency order is Profile, Source, Calendar; within each dependency it
    /// is SourceChanged, ProfileChanged, DependencyRetired, PolicyUndetermined.
    pub fn new(
        state: DeadlineReviewState,
        reasons: Vec<TrackingReviewRequirement>,
    ) -> Result<Self, TrackingReviewError> {
        if reasons.len() > 8 {
            return Err(TrackingReviewError::TooManyReasons);
        }
        let invalid_state = match state {
            DeadlineReviewState::Pending => reasons.is_empty(),
            DeadlineReviewState::Accepted | DeadlineReviewState::LegacyUndeclared => {
                !reasons.is_empty()
            }
        };
        if invalid_state {
            return Err(TrackingReviewError::InvalidStateReasons);
        }
        for requirement in &reasons {
            match (requirement.dependency, requirement.reason) {
                (TrackingDependency::Source, TrackingReviewReason::SourceChanged)
                | (TrackingDependency::Profile, TrackingReviewReason::ProfileChanged)
                | (_, TrackingReviewReason::DependencyRetired)
                | (_, TrackingReviewReason::PolicyUndetermined) => {}
                _ => return Err(TrackingReviewError::InvalidDependencyReason),
            }
        }
        if reasons.windows(2).any(|pair| key(pair[0]) >= key(pair[1])) {
            return Err(TrackingReviewError::NonCanonicalReasons);
        }
        Ok(Self { state, reasons })
    }

    pub const fn state(&self) -> DeadlineReviewState {
        self.state
    }

    pub fn reasons(&self) -> &[TrackingReviewRequirement] {
        &self.reasons
    }

    /// Validate policy meaning using presence already verified by the caller.
    /// Presence order is Profile, Source, Calendar. This method does not verify
    /// dependency identity, mandatory profile presence or actual head contents.
    pub fn validate_policies(
        &self,
        policies: &TrackingPolicies,
        present: [bool; 3],
    ) -> Result<(), TrackingReviewError> {
        let dependencies = [
            (TrackingDependency::Profile, policies.profile),
            (TrackingDependency::Source, policies.source),
            (TrackingDependency::Calendar, policies.calendar),
        ];
        if self.state == DeadlineReviewState::LegacyUndeclared {
            return if dependencies
                .iter()
                .all(|(_, policy)| *policy == TrackingPolicy::Undetermined)
            {
                Ok(())
            } else {
                Err(TrackingReviewError::LegacyPolicyDeclared)
            };
        }
        for ((dependency, policy), present) in dependencies.into_iter().zip(present) {
            self.validate_dependency(dependency, policy, present)?;
        }
        Ok(())
    }

    fn validate_dependency(
        &self,
        dependency: TrackingDependency,
        policy: TrackingPolicy,
        present: bool,
    ) -> Result<(), TrackingReviewError> {
        let requirements = || {
            self.reasons
                .iter()
                .filter(move |value| value.dependency == dependency)
        };
        if !present {
            return if policy == TrackingPolicy::Undetermined && requirements().next().is_none() {
                Ok(())
            } else {
                Err(TrackingReviewError::AbsentDependency)
            };
        }
        if self.state == DeadlineReviewState::Accepted && policy == TrackingPolicy::Undetermined {
            return Err(TrackingReviewError::UndeclaredAcceptedDependency);
        }
        if self.state == DeadlineReviewState::Pending
            && policy == TrackingPolicy::Undetermined
            && !requirements().any(|value| value.reason == TrackingReviewReason::PolicyUndetermined)
        {
            return Err(TrackingReviewError::MissingPolicyReason);
        }
        for value in requirements() {
            let valid = match value.reason {
                TrackingReviewReason::SourceChanged | TrackingReviewReason::ProfileChanged => {
                    policy == TrackingPolicy::Follow
                }
                TrackingReviewReason::PolicyUndetermined => policy == TrackingPolicy::Undetermined,
                TrackingReviewReason::DependencyRetired => true,
            };
            if !valid {
                return Err(TrackingReviewError::InvalidPolicyReason);
            }
        }
        Ok(())
    }
}

fn key(requirement: TrackingReviewRequirement) -> (u8, u8) {
    let dependency = match requirement.dependency {
        TrackingDependency::Profile => 0,
        TrackingDependency::Source => 1,
        TrackingDependency::Calendar => 2,
    };
    let reason = match requirement.reason {
        TrackingReviewReason::SourceChanged => 0,
        TrackingReviewReason::ProfileChanged => 1,
        TrackingReviewReason::DependencyRetired => 2,
        TrackingReviewReason::PolicyUndetermined => 3,
    };
    (dependency, reason)
}
