//! Typed cursor parsing followed by application query validation.

use application::participants::{
    ParticipantHistoryQuery, ParticipantQuery, ParticipantStatusFilter,
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

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ListQuery {
    limit: u32,
    after_id: Option<Uuid>,
    name: Option<String>,
    procedural_role: Option<String>,
    status: Status,
}

impl Default for ListQuery {
    fn default() -> Self {
        Self {
            limit: 50,
            after_id: None,
            name: None,
            procedural_role: None,
            status: Status::Active,
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
        ParticipantQuery::new(
            self.limit,
            self.after_id.map(ParticipantId::from_uuid),
            self.name.as_deref(),
            self.procedural_role.as_deref(),
            status,
        )
        .map_err(Into::into)
    }
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
