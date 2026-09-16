use super::{inconsistent, target};
use application::{
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    procedural_facts::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
use postgres::Row;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(inconsistent)
}
pub(super) fn counter(value: i64) -> Result<u32, ApplicationError> {
    u32::try_from(value).map_err(inconsistent)
}
pub(super) fn row(
    row: &Row,
    administration: CurrentCaseAdministration,
    hasher: &dyn DocumentHasher,
) -> Result<FactDetail, ApplicationError> {
    if row
        .try_get::<_, i64>("initial_revision")
        .map_err(inconsistent)?
        != 1
    {
        return Err(inconsistent("fact root first revision differs"));
    }
    let family: &str = row.try_get("family").map_err(inconsistent)?;
    let canonical: Vec<u8> = row.try_get("values_canonical").map_err(inconsistent)?;
    let values = crate::procedural_fact_codec::values(
        family,
        &canonical,
        &row.try_get("values_view").map_err(inconsistent)?,
    )?;
    let sources_bytes: Vec<u8> = row.try_get("sources_canonical").map_err(inconsistent)?;
    let sources = crate::procedural_fact_codec::sources(
        &sources_bytes,
        &row.try_get("sources_view").map_err(inconsistent)?,
    )?;
    let action = match row.try_get::<_, &str>("action").map_err(inconsistent)? {
        "record" => FactAction::Record,
        "correct" => FactAction::Correct,
        "withdraw" => FactAction::Withdraw,
        _ => return Err(inconsistent("invalid fact action")),
    };
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|raw| {
            let value = FactText::new(&raw).map_err(inconsistent)?;
            if value.as_str() != raw {
                return Err(inconsistent("noncanonical fact reason"));
            }
            Ok(value)
        })
        .transpose()?;
    let revision = FactRevision::new(counter(row.try_get("revision").map_err(inconsistent)?)?)
        .map_err(inconsistent)?;
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
        return Err(inconsistent("invalid fact capture clock or author"));
    }
    let metadata = FactRevisionMetadata {
        revision,
        values_digest: digest(row.try_get("values_digest").map_err(inconsistent)?)?,
        status: match row.try_get::<_, &str>("status").map_err(inconsistent)? {
            "recorded" => FactStatus::Recorded,
            "withdrawn" => FactStatus::Withdrawn,
            _ => return Err(inconsistent("invalid fact status")),
        },
        reason,
        receipt: FactReceipt {
            operation_id: FactOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            sources_digest: digest(row.try_get("sources_digest").map_err(inconsistent)?)?,
            submission_digest: digest(row.try_get("submission_digest").map_err(inconsistent)?)?,
        },
        recorded_administration: administration,
        recorded_at: at,
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email,
        },
    };
    let id: Uuid = row.try_get("id").map_err(inconsistent)?;
    let case = CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?);
    let parent: Option<Uuid> = row.try_get("parent_resolution_id").map_err(inconsistent)?;
    let parent_family: Option<&str> = row.try_get("parent_family").map_err(inconsistent)?;
    let snapshot = match (values, parent, parent_family) {
        (ProceduralFactValues::Resolution(values), None, None) => {
            ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
                root: ResolutionRoot::new(ResolutionId::from_uuid(id), case),
                metadata,
                values: *values,
            }))
        }
        (ProceduralFactValues::Notification(values), Some(parent), Some("resolution")) => {
            ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
                root: NotificationRoot::new(
                    NotificationId::from_uuid(id),
                    case,
                    ResolutionId::from_uuid(parent),
                ),
                metadata,
                values: *values,
            }))
        }
        _ => return Err(inconsistent("fact family or immutable parent differs")),
    };
    let detail = FactDetail { snapshot, sources };
    fact_receipt_matches(hasher, &detail)?;
    let m = detail.snapshot.metadata();
    let submission: Vec<u8> = row.try_get("submission_canonical").map_err(inconsistent)?;
    if fact_submission_bytes(
        m.recorded_by.id,
        case,
        &target::command(&detail.snapshot)?,
        m.values_digest,
        m.receipt.sources_digest,
    )? != submission
        || hasher.hash_bytes(&submission) != m.receipt.submission_digest
        || row
            .try_get::<_, serde_json::Value>("submission_view")
            .map_err(inconsistent)?
            != target::projection(&detail.snapshot)
    {
        return Err(inconsistent("fact submission bytes or projection differ"));
    }
    Ok(detail)
}
