use crate::error::ApiError;
use application::{case_reports::*, cases::CaseStatusFilter};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use uuid::Uuid;

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_case_report_request",
        "invalid report identity, filters or query",
    )
}
pub(super) fn uuid(value: &str) -> Result<Uuid, ApiError> {
    let id = Uuid::parse_str(value).map_err(|_| invalid())?;
    if id.is_nil() {
        return Err(invalid());
    }
    Ok(id)
}
pub(super) fn id(value: &str) -> Result<CaseReportId, ApiError> {
    Ok(CaseReportId::from_uuid(uuid(value)?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestBody {
    operation_id: String,
    filters: Filters,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    created_from: String,
    created_before: String,
    status: String,
    assigned_litigator: Option<String>,
}
impl RequestBody {
    pub(super) fn command(self) -> Result<CaseReportCommand, ApiError> {
        let parse = |s: &str| -> Result<OffsetDateTime, ApiError> {
            let at = OffsetDateTime::parse(s, &Rfc3339).map_err(|_| invalid())?;
            if at.offset() != UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
                return Err(invalid());
            }
            Ok(at)
        };
        let created_from = parse(&self.filters.created_from)?;
        let created_before = parse(&self.filters.created_before)?;
        if created_from >= created_before
            || created_before - created_from > time::Duration::days(366)
        {
            return Err(invalid());
        }
        let status = match self.filters.status.as_str() {
            "all" => CaseStatusFilter::All,
            "active" => CaseStatusFilter::Active,
            "closed" => CaseStatusFilter::Closed,
            _ => return Err(invalid()),
        };
        let assigned_litigator = self
            .filters
            .assigned_litigator
            .map(|s| uuid(&s).map(domain::identity::UserId::from_uuid))
            .transpose()?;
        Ok(CaseReportCommand {
            operation_id: CaseReportOperationId::from_uuid(uuid(&self.operation_id)?),
            filters: CaseReportFilters {
                created_from,
                created_before,
                status,
                assigned_litigator,
            },
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    limit: Option<u32>,
    after_id: Option<String>,
    unread_only: Option<bool>,
}
impl Page {
    pub(super) fn query(self) -> Result<CaseReportQuery, ApiError> {
        let limit = self.limit.unwrap_or(20);
        if !(1..=100).contains(&limit) {
            return Err(invalid());
        }
        Ok(CaseReportQuery {
            limit,
            after_id: self.after_id.map(|s| id(&s)).transpose()?,
            unread_only: self.unread_only.unwrap_or(false),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Download {
    format: String,
}
impl Download {
    pub(super) fn format(self) -> Result<CaseReportFormat, ApiError> {
        match self.format.as_str() {
            "pdf" => Ok(CaseReportFormat::Pdf),
            "csv" => Ok(CaseReportFormat::Csv),
            _ => Err(invalid()),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Litigators {
    limit: Option<u32>,
    after_id: Option<String>,
}
impl Litigators {
    pub(super) fn query(self) -> Result<CaseReportLitigatorQuery, ApiError> {
        let limit = self.limit.unwrap_or(20);
        if !(1..=100).contains(&limit) {
            return Err(invalid());
        }
        Ok(CaseReportLitigatorQuery {
            limit,
            after_id: self
                .after_id
                .map(|s| uuid(&s).map(domain::identity::UserId::from_uuid))
                .transpose()?,
        })
    }
}
