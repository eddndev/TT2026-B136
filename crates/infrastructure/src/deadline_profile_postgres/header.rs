use super::{inconsistent, projection};
use application::{deadline_profiles::*, ApplicationError};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    procedural_facts::FactText,
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
) -> Result<DeadlineProfileHistoryEntry, ApplicationError> {
    if row
        .try_get::<_, i64>("initial_revision")
        .map_err(inconsistent)?
        != 1
    {
        return Err(inconsistent("invalid profile initial revision"));
    }
    let (_, scope) = projection::read(
        &row.try_get::<_, serde_json::Value>("definition_view")
            .map_err(inconsistent)?,
    )?;
    let expected_case = match scope {
        DeadlineProfileScope::Global(_) => None,
        DeadlineProfileScope::Case(id) => Some(id.as_uuid()),
    };
    if row
        .try_get::<_, Option<uuid::Uuid>>("scope_case_id")
        .map_err(inconsistent)?
        != expected_case
    {
        return Err(inconsistent("profile root scope disagrees with definition"));
    }
    let algorithm = match row.try_get::<_, i16>("algorithm").map_err(inconsistent)? {
        1 => DeadlineProfileAlgorithm::V1,
        _ => return Err(inconsistent("unsupported profile algorithm")),
    };
    let revision =
        DeadlineProfileRevision::new(counter(row.try_get("revision").map_err(inconsistent)?)?)
            .map_err(inconsistent)?;
    let action = match row.try_get::<_, &str>("action").map_err(inconsistent)? {
        "publish" => DeadlineProfileAction::Publish,
        "replace" => DeadlineProfileAction::Replace,
        "retire" => DeadlineProfileAction::Retire,
        _ => return Err(inconsistent("invalid profile action")),
    };
    let status = match row.try_get::<_, &str>("status").map_err(inconsistent)? {
        "published" => DeadlineProfileStatus::Published,
        "retired" => DeadlineProfileStatus::Retired,
        _ => return Err(inconsistent("invalid profile status")),
    };
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|s| {
            let value = FactText::new(&s).map_err(inconsistent)?;
            if value.as_str() != s {
                return Err(inconsistent("noncanonical profile reason"));
            }
            Ok(value)
        })
        .transpose()?;
    let recorded_at = OffsetDateTime::from_unix_timestamp(
        row.try_get("recorded_at_seconds").map_err(inconsistent)?,
    )
    .map_err(inconsistent)?
    .replace_nanosecond(counter(i64::from(
        row.try_get::<_, i32>("recorded_at_nanoseconds")
            .map_err(inconsistent)?,
    ))?)
    .map_err(inconsistent)?;
    let email: String = row.try_get("recorded_by_email").map_err(inconsistent)?;
    if !(1..=9999).contains(&recorded_at.year())
        || email.is_empty()
        || email.trim() != email
        || email.chars().count() > 320
        || email.chars().any(char::is_control)
    {
        return Err(inconsistent("invalid profile capture time or author"));
    }
    let entry = DeadlineProfileHistoryEntry {
        id: DeadlineProfileId::from_uuid(row.try_get("profile_id").map_err(inconsistent)?),
        revision,
        status,
        algorithm,
        scope,
        definition_digest: digest(row.try_get("definition_digest").map_err(inconsistent)?)?,
        reason,
        receipt: DeadlineProfileReceipt {
            operation_id: DeadlineProfileOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            submission_digest: digest(row.try_get("submission_digest").map_err(inconsistent)?)?,
        },
        recorded_at,
        recorded_by: DeadlineProfileActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email,
        },
    };
    deadline_profile_history_receipt_matches(hasher, &entry)?;
    let bytes: Vec<u8> = row.try_get("submission_canonical").map_err(inconsistent)?;
    if hasher.hash_bytes(&bytes) != entry.receipt.submission_digest
        || row
            .try_get::<_, serde_json::Value>("submission_view")
            .map_err(inconsistent)?
            != projection::receipt(&entry)
    {
        return Err(inconsistent(
            "profile submission bytes or projection differ",
        ));
    }
    Ok(entry)
}
