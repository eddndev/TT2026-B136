use super::{AgendaCursor, AgendaQuery};
use crate::{deadlines::DeadlineOverview, hearings::HearingOverview, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::{HearingModality, HearingTime},
    procedural_resources::ResourceId,
    resource_activities::ResourceActivityId,
    resource_hearings::{ResourceHearingId, ResourceHearingKind, ResourceHearingRevision},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgendaCaseSummary {
    pub case_id: CaseId,
    pub title: String,
    pub reference: String,
    pub status: CaseAdministrativeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHearingAgendaOverview {
    pub case_id: CaseId,
    pub resource_id: ResourceId,
    pub id: ResourceHearingId,
    pub revision: ResourceHearingRevision,
    pub kind: ResourceHearingKind,
    pub scheduled_at: HearingTime,
    pub modality: HearingModality,
    pub participant_count: u8,
    pub association_id: ResourceActivityId,
    pub capture_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgendaItem {
    Hearing(HearingOverview),
    Deadline {
        case: AgendaCaseSummary,
        deadline: Box<DeadlineOverview>,
    },
    ResourceHearing {
        case: AgendaCaseSummary,
        hearing: Box<ResourceHearingAgendaOverview>,
    },
}
impl AgendaItem {
    pub fn key(&self) -> Result<AgendaCursor, ApplicationError> {
        super::validation::item_key(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgendaPage {
    pub checked_at: OffsetDateTime,
    pub items: Vec<AgendaItem>,
    pub complete: bool,
    pub next_after: Option<AgendaCursor>,
}
impl AgendaPage {
    pub fn validate(&self, query: &AgendaQuery) -> Result<(), ApplicationError> {
        super::validation::validate_page(self, query)
    }
}
