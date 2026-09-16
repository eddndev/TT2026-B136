use super::{inconsistent, projection};
use application::{judicial_calendars::*, ApplicationError};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
use postgres::Row;
use time::OffsetDateTime;

pub(super) fn counter(value: i64) -> Result<u32, ApplicationError> {
    u32::try_from(value).map_err(inconsistent)
}
fn digest(value: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&value).map_err(inconsistent)
}
pub(super) fn row(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<JudicialCalendarDetail, ApplicationError> {
    if row
        .try_get::<_, i64>("initial_revision")
        .map_err(inconsistent)?
        != 1
    {
        return Err(inconsistent("calendar root has invalid initial revision"));
    }
    let bytes: Vec<u8> = row.try_get("values_canonical").map_err(inconsistent)?;
    let values = JudicialCalendarValues::from_canonical_bytes(&bytes).map_err(inconsistent)?;
    if row
        .try_get::<_, serde_json::Value>("values_view")
        .map_err(inconsistent)?
        != projection::values(&values)
    {
        return Err(inconsistent(
            "calendar projection differs from canonical values",
        ));
    }
    let revision =
        JudicialCalendarRevision::new(counter(row.try_get("revision").map_err(inconsistent)?)?)
            .map_err(inconsistent)?;
    let action = match row.try_get::<_, &str>("action").map_err(inconsistent)? {
        "publish" => JudicialCalendarAction::Publish,
        "replace" => JudicialCalendarAction::Replace,
        "retire" => JudicialCalendarAction::Retire,
        _ => return Err(inconsistent("invalid calendar action")),
    };
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|raw| {
            let reason = JudicialCalendarReason::new(&raw).map_err(inconsistent)?;
            if reason.as_str() != raw {
                return Err(inconsistent("noncanonical calendar reason"));
            }
            Ok(reason)
        })
        .transpose()?;
    let at = OffsetDateTime::from_unix_timestamp(
        row.try_get("recorded_at_seconds").map_err(inconsistent)?,
    )
    .map_err(inconsistent)?
    .replace_nanosecond(counter(i64::from(
        row.try_get::<_, i32>("recorded_at_nanoseconds")
            .map_err(inconsistent)?,
    ))?)
    .map_err(inconsistent)?;
    let email: String = row.try_get("recorded_by_email").map_err(inconsistent)?;
    if !(1..=9999).contains(&at.year())
        || email.trim() != email
        || email.is_empty()
        || email.chars().count() > 320
        || email.chars().any(char::is_control)
    {
        return Err(inconsistent("invalid calendar capture time or author"));
    }
    let detail = JudicialCalendarDetail {
        id: JudicialCalendarId::from_uuid(row.try_get("calendar_id").map_err(inconsistent)?),
        revision,
        values,
        values_digest: digest(row.try_get("values_digest").map_err(inconsistent)?)?,
        status: row
            .try_get::<_, &str>("status")
            .map_err(inconsistent)?
            .parse()
            .map_err(inconsistent)?,
        reason,
        receipt: JudicialCalendarReceipt {
            operation_id: JudicialCalendarOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            submission_digest: digest(row.try_get("submission_digest").map_err(inconsistent)?)?,
        },
        recorded_at: at,
        recorded_by: JudicialCalendarActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email,
        },
    };
    judicial_calendar_receipt_matches(hasher, &detail)?;
    let submission: Vec<u8> = row.try_get("submission_canonical").map_err(inconsistent)?;
    if judicial_calendar_submission_bytes(
        detail.recorded_by.id,
        &command(&detail)?,
        detail.values_digest,
    ) != submission
        || hasher.hash_bytes(&submission) != detail.receipt.submission_digest
        || row
            .try_get::<_, serde_json::Value>("submission_view")
            .map_err(inconsistent)?
            != projection::receipt(&detail)
    {
        return Err(inconsistent("calendar receipt bytes or projection differ"));
    }
    Ok(detail)
}
fn command(v: &JudicialCalendarDetail) -> Result<JudicialCalendarCommand, ApplicationError> {
    let change = match v.receipt.action {
        JudicialCalendarAction::Publish => JudicialCalendarChange::Publish {
            values: v.values.clone(),
        },
        action => {
            let expected_revision =
                JudicialCalendarRevision::new(v.receipt.expected_revision).map_err(inconsistent)?;
            let reason = v
                .reason
                .clone()
                .ok_or_else(|| inconsistent("calendar change reason missing"))?;
            if action == JudicialCalendarAction::Replace {
                JudicialCalendarChange::Replace {
                    expected_revision,
                    values: v.values.clone(),
                    reason,
                }
            } else {
                JudicialCalendarChange::Retire {
                    expected_revision,
                    reason,
                }
            }
        }
    };
    Ok(JudicialCalendarCommand {
        operation_id: v.receipt.operation_id,
        calendar_id: v.id,
        change,
    })
}
