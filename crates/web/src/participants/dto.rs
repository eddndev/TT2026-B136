//! Strict participant input and captured snapshot representations.

use crate::typed_participants::projection;
use application::participants::{
    DirectoryStatus, ParticipantDetail, ParticipantHistoryPage, ParticipantPage,
    ParticipantRevision, ParticipantSnapshot, ParticipantValues,
};
use application::ApplicationError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
#[serde(transparent)]
pub(super) struct SnapshotResponse(Value);

impl TryFrom<ParticipantSnapshot> for SnapshotResponse {
    type Error = ApiError;
    fn try_from(row: ParticipantSnapshot) -> Result<Self, Self::Error> {
        projection::manual(row).map(Self)
    }
}
impl TryFrom<ParticipantDetail> for SnapshotResponse {
    type Error = ApiError;
    fn try_from(row: ParticipantDetail) -> Result<Self, Self::Error> {
        projection::detail(row).map(Self)
    }
}

#[derive(Serialize)]
pub(super) struct PageResponse {
    participants: Vec<Value>,
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
                .map(projection::overview)
                .collect(),
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
