use crate::error::ApiError;
use application::{cases::CurrentCaseAdministration, procedural_facts::*};
use domain::{case_administration::CaseAdministrativeStatus, cases::CaseId};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

pub(super) fn administration(
    current: &CurrentCaseAdministration,
    case: CaseId,
) -> Result<Value, ApiError> {
    match current {
        CurrentCaseAdministration::Unrevised(metadata) => Ok(json!({
            "kind":"unrevised", "title":metadata.title(),
            "reference":metadata.reference(), "status":"active"
        })),
        CurrentCaseAdministration::Recorded(snapshot) => {
            if snapshot.case_id != case
                || snapshot.values.status() != CaseAdministrativeStatus::Active
            {
                return Err(ApiError::internal());
            }
            Ok(json!({"kind":"recorded", "case_id":case,
                "revision":snapshot.revision.get(),
                "title":snapshot.values.metadata().title(),
                "reference":snapshot.values.metadata().reference(),
                "status":snapshot.values.status().as_str(),
                "values_digest":snapshot.values_digest.to_hex(),
                "changed_at":utc(snapshot.changed_at)?,
                "changed_by":{"id":snapshot.changed_by.id,"email":snapshot.changed_by.email}
            }))
        }
    }
}

pub(super) fn metadata(value: &FactRevisionMetadata, case: CaseId) -> Result<Value, ApiError> {
    let receipt = &value.receipt;
    let valid = match receipt.action {
        FactAction::Record => receipt.expected_revision == 0 && value.reason.is_none(),
        FactAction::Correct | FactAction::Withdraw => {
            receipt.expected_revision > 0 && value.reason.is_some()
        }
    };
    if !valid
        || receipt.expected_revision.checked_add(1) != Some(value.revision.get())
        || value.status != receipt.action.resulting_status()
    {
        return Err(ApiError::internal());
    }
    Ok(json!({
        "revision":value.revision.get(), "values_digest":value.values_digest.to_hex(),
        "status":status(value.status), "reason":value.reason.as_ref().map(FactText::as_str),
        "receipt":{"operation_id":receipt.operation_id.to_string(),
            "action":action(receipt.action), "expected_revision":receipt.expected_revision,
            "sources_digest":receipt.sources_digest.to_hex(),
            "submission_digest":receipt.submission_digest.to_hex()},
        "recorded_administration":administration(&value.recorded_administration,case)?,
        "recorded_at":utc(value.recorded_at)?,
        "recorded_by":{"id":value.recorded_by.id,"email":value.recorded_by.email}
    }))
}

pub(super) const fn status(value: FactStatus) -> &'static str {
    match value {
        FactStatus::Recorded => "recorded",
        FactStatus::Withdrawn => "withdrawn",
    }
}
fn action(value: FactAction) -> &'static str {
    match value {
        FactAction::Record => "record",
        FactAction::Correct => "correct",
        FactAction::Withdraw => "withdraw",
    }
}
fn utc(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .checked_to_offset(UtcOffset::UTC)
        .filter(|v| (1..=9999).contains(&v.year()))
        .ok_or_else(ApiError::internal)?
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
