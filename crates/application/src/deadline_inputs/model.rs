use crate::{
    cases::CurrentCaseAdministration, hearing_results::HearingResultDetail,
    judicial_calendars::JudicialCalendarDetail, procedural_facts::FactDetail,
};
use domain::{
    cases::CaseId,
    deadline_arithmetic::ArithmeticRule,
    deadline_triggers::{TriggerRequirement, TriggerSelection},
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineCalendarRef {
    pub id: JudicialCalendarId,
    pub revision: JudicialCalendarRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineInputRequest {
    pub trigger: TriggerSelection,
    pub requirement: TriggerRequirement,
    pub rule: ArithmeticRule,
    pub calendar: Option<DeadlineCalendarRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineSourceDetail {
    Fact(Box<FactDetail>),
    HearingResult(Box<HearingResultDetail>),
}

/// Exact selections and heads observed together; statuses do not establish eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineInputMaterial {
    pub case_id: CaseId,
    pub administration: CurrentCaseAdministration,
    pub source: Option<DeadlineSourceDetail>,
    pub source_head: Option<DeadlineSourceDetail>,
    pub calendar: Option<JudicialCalendarDetail>,
    pub calendar_head: Option<JudicialCalendarDetail>,
}

#[derive(Debug, thiserror::Error)]
pub enum DeadlineInputError {
    #[error("inconsistent deadline inputs: {0}")]
    Inconsistent(String),
}
