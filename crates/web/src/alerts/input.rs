use crate::error::ApiError;
use application::{alerts::*, ApplicationError};
use axum::extract::Request;
use domain::alerts::{AlertAnticipations, AlertLeadHours};
use serde::{de::DeserializeOwned, Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageQuery {
    limit: Option<u32>,
    read: Option<String>,
    state: Option<String>,
    cursor: Option<String>,
}
impl PageQuery {
    pub fn validate(self) -> Result<AlertQuery, ApiError> {
        let read = match self.read.as_deref().unwrap_or("all") {
            "all" => AlertReadFilter::All,
            "unread" => AlertReadFilter::Unread,
            _ => return Err(invalid("read filter")),
        };
        let state = match self.state.as_deref().unwrap_or("active") {
            "active" => AlertStateFilter::Active,
            "all" => AlertStateFilter::All,
            _ => return Err(invalid("state filter")),
        };
        let cursor = self.cursor.as_deref().map(AlertCursor::parse).transpose()?;
        AlertQuery::new(self.limit.unwrap_or(20), read, state, cursor).map_err(Into::into)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preferences {
    operation_id: String,
    expected_revision: u32,
    values: Values,
}
impl Preferences {
    pub fn validate(self) -> Result<AlertPreferenceCommand, ApiError> {
        Ok(AlertPreferenceCommand {
            operation_id: AlertOperationId::from_uuid(uuid(&self.operation_id)?),
            expected_revision: self.expected_revision,
            values: self.values.validate()?,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Values {
    hearing_upcoming: Family,
    deadline_upcoming: Family,
    overdue_unattended: Channels,
    review_required: Channels,
    due_changed_soon: Channels,
}
impl Values {
    fn validate(self) -> Result<AlertPreferenceValues, ApiError> {
        Ok(AlertPreferenceValues {
            hearing_upcoming: self.hearing_upcoming.validate()?,
            deadline_upcoming: self.deadline_upcoming.validate()?,
            overdue_unattended: self.overdue_unattended.into(),
            review_required: self.review_required.into(),
            due_changed_soon: self.due_changed_soon.into(),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Family {
    lead_hours: Vec<u16>,
    channels: Channels,
}
impl Family {
    fn validate(self) -> Result<AlertFamilyPreferences, ApiError> {
        Ok(AlertFamilyPreferences {
            anticipations: AlertAnticipations::new(
                self.lead_hours
                    .into_iter()
                    .map(AlertLeadHours::new)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| {
                        ApiError::from(ApplicationError::Alert(AlertError::Timing(error)))
                    })?,
            )
            .map_err(|error| ApiError::from(ApplicationError::Alert(AlertError::Timing(error))))?,
            channels: self.channels.into(),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Channels {
    internal: bool,
    email: bool,
}
impl From<Channels> for AlertChannels {
    fn from(value: Channels) -> Self {
        Self {
            internal: value.internal,
            email: value.email,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Read {
    operation_id: String,
}
impl Read {
    pub fn validate(self) -> Result<AlertOperationId, ApiError> {
        Ok(AlertOperationId::from_uuid(uuid(&self.operation_id)?))
    }
}
pub(super) async fn body<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    crate::request::json::read(
        request,
        16 * 1024,
        "alert_body_too_large",
        "alert command exceeds 16 KiB",
    )
    .await
}
pub(super) fn id(value: &str) -> Result<AlertId, ApiError> {
    Ok(AlertId::from_uuid(uuid(value)?))
}
fn uuid(value: &str) -> Result<Uuid, ApiError> {
    let id = Uuid::parse_str(value).map_err(|_| malformed())?;
    if id.to_string() != value {
        return Err(malformed());
    }
    Ok(id)
}
fn invalid(field: &'static str) -> ApiError {
    ApplicationError::Alert(AlertError::Invalid(field)).into()
}
pub(super) fn malformed() -> ApiError {
    ApiError::invalid_body("invalid_alert", "invalid alert request")
}
