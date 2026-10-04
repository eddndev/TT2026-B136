//! Prospective review of an ordinary hearing result and its configured consequence.
//!
//! No source capture or deadline receipt is fabricated during preparation.
//! See docs/adr/0070-prospective-hearing-derived-deadlines.md.
mod canonical;
mod capture;
mod finalization;
mod history;
mod history_validation;
mod port;
mod preparation;
mod prepared;
mod recorded_source;
mod service;

use crate::{
    deadline_evaluations::ProfiledDeadlineEvaluation,
    deadline_profiles::DeadlineProfileDetail,
    deadlines::{DeadlineError, DeadlineHumanCommand, DeadlineResponsibleSnapshot},
    hearing_results::{HearingResultCommand, HearingResultDraft},
    identity::Principal,
    judicial_calendars::JudicialCalendarDetail,
    ApplicationError,
};
use domain::crypto::Sha256Digest;

pub use capture::{hearing_derived_deadline_capture_bytes, HearingDerivedDeadlineCreation};
pub use finalization::finalize_hearing_derived_deadline;
pub use history::{
    restore_hearing_derived_deadline, HearingDerivedDeadlineEvidence, HearingDerivedDeadlineRecord,
};
pub use port::*;
pub use preparation::prepare_hearing_derived_deadline;
pub use prepared::PreparedHearingDerivedDeadline;
pub use service::HearingDerivedDeadlineService;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineCommand {
    pub result: HearingResultCommand,
    pub deadline: DeadlineHumanCommand,
}

/// Inputs resolved by authorized services; this envelope establishes no access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineMaterial {
    pub result: HearingResultDraft,
    pub profile: DeadlineProfileDetail,
    pub profile_head: DeadlineProfileDetail,
    pub calendar: Option<JudicialCalendarDetail>,
    pub calendar_head: Option<JudicialCalendarDetail>,
    pub responsible: DeadlineResponsibleSnapshot,
}

/// Immutable review, not a reservation, persisted source or final deadline receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineDraft {
    command: HearingDerivedDeadlineCommand,
    material: HearingDerivedDeadlineMaterial,
    actor: Principal,
    evaluation: ProfiledDeadlineEvaluation,
    review_digest: Sha256Digest,
}

impl HearingDerivedDeadlineDraft {
    pub fn command(&self) -> &HearingDerivedDeadlineCommand {
        &self.command
    }
    pub fn result(&self) -> &HearingResultDraft {
        &self.material.result
    }
    pub fn material(&self) -> &HearingDerivedDeadlineMaterial {
        &self.material
    }
    pub fn actor(&self) -> &Principal {
        &self.actor
    }
    pub fn evaluation(&self) -> &ProfiledDeadlineEvaluation {
        &self.evaluation
    }
    pub const fn review_digest(&self) -> Sha256Digest {
        self.review_digest
    }
    pub fn require_review(&self, expected: Sha256Digest) -> Result<(), ApplicationError> {
        if self.review_digest != expected {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        Ok(())
    }
}

fn invalid(field: &'static str) -> ApplicationError {
    DeadlineError::Invalid(field).into()
}
