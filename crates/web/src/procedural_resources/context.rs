use crate::error::ApiError;
use application::{
    case_stages::{CaseStageDetail, CaseStageEntry, CurrentCaseStage},
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
};
use domain::cases::CaseId;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
pub(crate) fn actor(v: &CaseActorSnapshot) -> Result<Value, ApiError> {
    if v.email.is_empty() || v.email.trim() != v.email || v.email.chars().any(char::is_control) {
        return Err(ApiError::internal());
    }
    Ok(json!({"id":v.id,"email":v.email}))
}
pub(crate) fn utc(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .checked_to_offset(UtcOffset::UTC)
        .filter(|v| (1..=9999).contains(&v.year()))
        .ok_or_else(ApiError::internal)?
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
pub(crate) fn administration(
    v: &CurrentCaseAdministration,
    case: CaseId,
) -> Result<Value, ApiError> {
    if let Some(s) = v.snapshot() {
        actor(&s.changed_by)?;
    }
    crate::procedural_facts::projection::administration(v, case)
}
pub(super) fn stage(
    v: &CurrentCaseStage,
    admin: &CurrentCaseAdministration,
    case: CaseId,
) -> Result<Value, ApiError> {
    if let Some(entry) = v.entry() {
        let snapshot = admin.snapshot().ok_or_else(ApiError::internal)?;
        let (revision, digest) = match entry {
            CaseStageEntry::Initial(s) => (s.administration_revision, s.administration_digest),
            CaseStageEntry::Changed(s) => (s.administration_revision, s.administration_digest),
        };
        if revision > snapshot.revision
            || (revision == snapshot.revision && digest != snapshot.values_digest)
        {
            return Err(ApiError::internal());
        }
        actor(entry.recorded_by())?;
        utc(entry.recorded_at())?;
    }
    let value = crate::case_stages::response::Detail::from_row(
        CaseStageDetail {
            case_id: case,
            current: v.clone(),
        },
        case,
    )?;
    serde_json::to_value(value).map_err(|_| ApiError::internal())
}
