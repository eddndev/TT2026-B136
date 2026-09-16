//! Typed cursor parsing followed by application query validation.

use application::participants::{
    ParticipantHistoryQuery, ParticipantProfileFilter, ParticipantQuery, ParticipantStatusFilter,
};
use serde::Deserialize;
use uuid::Uuid;

use super::ParticipantId;
use crate::error::ApiError;

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    #[default]
    Active,
    Archived,
    All,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Profile {
    #[default]
    All,
    Manual,
    Typed,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ListQuery {
    limit: u32,
    after_id: Option<Uuid>,
    name: Option<String>,
    procedural_role: Option<String>,
    status: Status,
    kind: Option<String>,
    profile: Profile,
}

impl Default for ListQuery {
    fn default() -> Self {
        Self {
            limit: 50,
            after_id: None,
            name: None,
            procedural_role: None,
            status: Status::Active,
            kind: None,
            profile: Profile::All,
        }
    }
}

impl ListQuery {
    pub fn validate(self) -> Result<ParticipantQuery, ApiError> {
        let status = match self.status {
            Status::Active => ParticipantStatusFilter::Active,
            Status::Archived => ParticipantStatusFilter::Archived,
            Status::All => ParticipantStatusFilter::All,
        };
        let query = ParticipantQuery::new(
            self.limit,
            self.after_id.map(ParticipantId::from_uuid),
            self.name.as_deref(),
            self.procedural_role.as_deref(),
            status,
        )?;
        let kind = self.kind.as_deref().map(kind).transpose()?;
        let profile = match self.profile {
            Profile::All => ParticipantProfileFilter::All,
            Profile::Manual => ParticipantProfileFilter::Manual,
            Profile::Typed => ParticipantProfileFilter::Typed,
        };
        Ok(query.with_profile_filter(kind, profile))
    }
}

fn kind(value: &str) -> Result<domain::typed_participants::ParticipantKind, ApiError> {
    use domain::typed_participants::ParticipantKind as K;
    Ok(match value {
        "defendant" => K::Defendant,
        "victim" => K::Victim,
        "defense_counsel" => K::DefenseCounsel,
        "prosecutor" => K::Prosecutor,
        "victim_counsel" => K::VictimCounsel,
        "control_judge" => K::ControlJudge,
        "trial_court" => K::TrialCourt,
        "expert" => K::Expert,
        "police" => K::Police,
        "precautionary_supervisor" => K::PrecautionarySupervisor,
        "other" => K::Other,
        _ => {
            return Err(ApiError::invalid_body(
                "invalid_query",
                "invalid participant kind filter",
            ))
        }
    })
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct HistoryQuery {
    limit: u32,
    before_revision: Option<u32>,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            limit: 50,
            before_revision: None,
        }
    }
}

impl HistoryQuery {
    pub fn validate(self) -> Result<ParticipantHistoryQuery, ApiError> {
        ParticipantHistoryQuery::new(self.limit, self.before_revision).map_err(Into::into)
    }
}
