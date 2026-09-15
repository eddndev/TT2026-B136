//! Strict participant input and captured snapshot representations.

use application::participants::{
    DirectoryStatus, ParticipantHistoryPage, ParticipantPage, ParticipantRevision,
    ParticipantSnapshot, ParticipantValues,
};
use application::ApplicationError;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, UtcOffset};

use crate::error::ApiError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateRequest {
    pub display_name: String,
    pub procedural_role: String,
    pub organization: Option<String>,
    pub legal_status: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplaceRequest {
    expected_revision: u32,
    display_name: String,
    procedural_role: String,
    organization: Option<String>,
    legal_status: Option<String>,
    directory_status: DirectoryStatus,
}

impl ReplaceRequest {
    pub fn validate(self) -> Result<(ParticipantRevision, ParticipantValues), ApiError> {
        let expected =
            ParticipantRevision::new(self.expected_revision).map_err(ApplicationError::from)?;
        let values = ParticipantValues::new(
            &self.display_name,
            &self.procedural_role,
            self.organization.as_deref(),
            self.legal_status.as_deref(),
            self.directory_status,
        )
        .map_err(ApplicationError::from)?;
        Ok((expected, values))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StatusRequest {
    expected_revision: u32,
    directory_status: DirectoryStatus,
}

impl StatusRequest {
    pub fn validate(self) -> Result<(ParticipantRevision, DirectoryStatus), ApiError> {
        Ok((
            ParticipantRevision::new(self.expected_revision).map_err(ApplicationError::from)?,
            self.directory_status,
        ))
    }
}

#[derive(Serialize)]
struct ActorResponse {
    id: String,
    email: String,
}

#[derive(Serialize)]
pub(super) struct SnapshotResponse {
    case_id: String,
    id: String,
    revision: u32,
    display_name: String,
    procedural_role: String,
    organization: Option<String>,
    legal_status: Option<String>,
    directory_status: DirectoryStatus,
    values_digest: String,
    changed_at: String,
    changed_by: ActorResponse,
}

impl TryFrom<ParticipantSnapshot> for SnapshotResponse {
    type Error = ApiError;
    fn try_from(row: ParticipantSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            case_id: row.case_id.to_string(),
            id: row.id.to_string(),
            revision: row.revision.get(),
            display_name: row.values.display_name().into(),
            procedural_role: row.values.procedural_role().into(),
            organization: row.values.organization().map(str::to_owned),
            legal_status: row.values.legal_status().map(str::to_owned),
            directory_status: row.values.directory_status(),
            values_digest: row.values_digest.to_hex(),
            changed_at: row
                .changed_at
                .to_offset(UtcOffset::UTC)
                .format(&Rfc3339)
                .map_err(|_| ApiError::internal())?,
            changed_by: ActorResponse {
                id: row.changed_by.id.to_string(),
                email: row.changed_by.email,
            },
        })
    }
}

#[derive(Serialize)]
pub(super) struct PageResponse {
    participants: Vec<SnapshotResponse>,
    has_more: bool,
    next_after_id: Option<String>,
}

impl TryFrom<ParticipantPage> for PageResponse {
    type Error = ApiError;
    fn try_from(page: ParticipantPage) -> Result<Self, Self::Error> {
        Ok(Self {
            participants: page
                .participants
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            has_more: page.has_more,
            next_after_id: page.next_after_id.map(|id| id.to_string()),
        })
    }
}

#[derive(Serialize)]
pub(super) struct HistoryResponse {
    revisions: Vec<SnapshotResponse>,
    has_more: bool,
    next_before_revision: Option<u32>,
}

impl TryFrom<ParticipantHistoryPage> for HistoryResponse {
    type Error = ApiError;
    fn try_from(page: ParticipantHistoryPage) -> Result<Self, Self::Error> {
        Ok(Self {
            revisions: page
                .revisions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            has_more: page.has_more,
            next_before_revision: page.next_before_revision.map(|revision| revision.get()),
        })
    }
}
