use application::cases::{
    CaseAdministrationHistoryQuery, CaseAdministrationQuery, CaseProfileFilter, CaseStatusFilter,
};
use domain::cases::CaseId;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    #[default]
    Active,
    Closed,
    All,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Profile {
    Complete,
    Pending,
    #[default]
    All,
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct IndexQuery {
    limit: u32,
    after_id: Option<String>,
    status: Status,
    profile: Profile,
    title: Option<String>,
    nuc: Option<String>,
    judicial_case_number: Option<String>,
}
impl Default for IndexQuery {
    fn default() -> Self {
        Self {
            limit: 50,
            after_id: None,
            status: Status::Active,
            profile: Profile::All,
            title: None,
            nuc: None,
            judicial_case_number: None,
        }
    }
}
impl IndexQuery {
    pub fn validate(self) -> Result<CaseAdministrationQuery, ApiError> {
        let after = self
            .after_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| ApiError::invalid_body("invalid_query", "after_id must be a uuid"))?
            .map(CaseId::from_uuid);
        let status = match self.status {
            Status::Active => CaseStatusFilter::Active,
            Status::Closed => CaseStatusFilter::Closed,
            Status::All => CaseStatusFilter::All,
        };
        let profile = match self.profile {
            Profile::Complete => CaseProfileFilter::Complete,
            Profile::Pending => CaseProfileFilter::Pending,
            Profile::All => CaseProfileFilter::All,
        };
        Ok(CaseAdministrationQuery::new(
            self.limit,
            after,
            status,
            profile,
            self.title.as_deref(),
            self.nuc.as_deref(),
            self.judicial_case_number.as_deref(),
        )?)
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
    pub fn validate(self) -> Result<CaseAdministrationHistoryQuery, ApiError> {
        Ok(CaseAdministrationHistoryQuery::new(
            self.limit,
            self.before_revision,
        )?)
    }
}
