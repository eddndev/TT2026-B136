//! Declared procedural stages, exact documentary references and bounded values.

mod adoption;
mod canonical;
mod change;
mod stage;
mod support;
mod text;
mod time;
mod transition;

pub use crate::case_administration::CaseStageRevision;
pub use ::time::{Date, OffsetDateTime, UtcOffset};
pub use adoption::StageAdoption;
pub use change::CaseStageChange;
pub use stage::CaseStage;
pub use support::StageSupportRef;
pub use text::{StageCourt, StageNote, StageReceiptReference};
pub use time::{DeclaredStagePrecision, DeclaredStageTime};
pub use transition::{IntermediateStageTransition, StageTransition, TrialStageTransition};
