use super::{request::parse_uuid, time::parse_time};
use crate::error::ApiError;
use application::hearings::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<u32>,
    after_id: Option<String>,
    status: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<HearingQuery, ApiError> {
        HearingQuery::new(
            self.limit.unwrap_or(20),
            self.after_id
                .map(|s| parse_uuid(&s, "invalid_hearing_id").map(HearingId::from_uuid))
                .transpose()?,
            status(self.status.as_deref(), HearingStatusFilter::All)?,
        )
        .map_err(|_| invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryQuery {
    limit: Option<u32>,
    before_revision: Option<u32>,
}
impl HistoryQuery {
    pub fn validate(self) -> Result<HearingHistoryQuery, ApiError> {
        HearingHistoryQuery::new(self.limit.unwrap_or(20), self.before_revision)
            .map_err(|_| invalid())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgendaQuery {
    from: String,
    until: String,
    limit: Option<u32>,
    status: Option<String>,
    after_time: Option<String>,
    after_id: Option<String>,
}
impl AgendaQuery {
    pub fn validate(self) -> Result<HearingAgendaQuery, ApiError> {
        let from = parse_time(&self.from).map_err(|_| invalid())?;
        let until = parse_time(&self.until).map_err(|_| invalid())?;
        let after = match (self.after_time, self.after_id) {
            (None, None) => None,
            (Some(at), Some(id)) => Some(HearingAgendaCursor {
                at: parse_time(&at).map_err(|_| invalid())?,
                id: HearingId::from_uuid(parse_uuid(&id, "invalid_hearing_id")?),
            }),
            _ => return Err(invalid()),
        };
        HearingAgendaQuery::new(
            self.limit.unwrap_or(20),
            from.value(),
            until.value(),
            after,
            status(self.status.as_deref(), HearingStatusFilter::Scheduled)?,
        )
        .map_err(|_| invalid())
    }
}
fn status(
    value: Option<&str>,
    default: HearingStatusFilter,
) -> Result<HearingStatusFilter, ApiError> {
    match value {
        None => Ok(default),
        Some("all") => Ok(HearingStatusFilter::All),
        Some("scheduled") => Ok(HearingStatusFilter::Scheduled),
        Some("cancelled") => Ok(HearingStatusFilter::Cancelled),
        _ => Err(invalid()),
    }
}
pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid hearing query parameters")
}
