use super::{attention, inconsistent};
use application::{
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision},
    deadlines::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    identity::UserId,
    procedural_facts::{FactLabel, FactText},
};
use postgres::Row;
use time::OffsetDateTime;

pub(super) struct Header {
    pub id: DeadlineId,
    pub case_id: CaseId,
    pub revision: DeadlineRevision,
    pub title: FactLabel,
    pub profile: DeadlineProfileRef,
    pub responsible: DeadlineResponsibleSnapshot,
    pub attention: DeadlineAttention,
    pub status: DeadlineStatus,
    pub reason: Option<FactText>,
    pub receipt: DeadlineReceipt,
    pub recorded_at: OffsetDateTime,
    pub recorded_by: DeadlineActorSnapshot,
}
pub(super) fn counter(value: i64) -> Result<u32, ApplicationError> {
    u32::try_from(value).map_err(inconsistent)
}
pub(super) fn digest(value: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&value).map_err(inconsistent)
}
fn email(raw: String) -> Result<String, ApplicationError> {
    if raw.is_empty()
        || raw.trim() != raw
        || raw.chars().count() > 320
        || raw.chars().any(char::is_control)
    {
        return Err(inconsistent("noncanonical captured email"));
    }
    Ok(raw)
}
pub(super) fn row(row: &Row) -> Result<Header, ApplicationError> {
    if row
        .try_get::<_, i64>("initial_revision")
        .map_err(inconsistent)?
        != 1
    {
        return Err(inconsistent("deadline root has invalid initial revision"));
    }
    let title: String = row.try_get("title").map_err(inconsistent)?;
    let title_value = FactLabel::new(&title).map_err(inconsistent)?;
    if title_value.as_str() != title {
        return Err(inconsistent("noncanonical deadline title"));
    }
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|raw| {
            let value = FactText::new(&raw).map_err(inconsistent)?;
            if value.as_str() != raw {
                return Err(inconsistent("noncanonical deadline reason"));
            }
            Ok(value)
        })
        .transpose()?;
    let revision = DeadlineRevision::new(counter(row.try_get("revision").map_err(inconsistent)?)?)
        .map_err(inconsistent)?;
    let action = match row.try_get::<_, &str>("action").map_err(inconsistent)? {
        "register" => DeadlineAction::Register,
        "correct" => DeadlineAction::Correct,
        "set_attention" => DeadlineAction::SetAttention,
        "retire" => DeadlineAction::Retire,
        _ => return Err(inconsistent("unknown deadline action")),
    };
    let recorded_at = OffsetDateTime::from_unix_timestamp(
        row.try_get("recorded_at_seconds").map_err(inconsistent)?,
    )
    .map_err(inconsistent)?
    .replace_nanosecond(counter(i64::from(
        row.try_get::<_, i32>("recorded_at_nanoseconds")
            .map_err(inconsistent)?,
    ))?)
    .map_err(inconsistent)?;
    if !(1..=9999).contains(&recorded_at.year()) {
        return Err(inconsistent("deadline capture year is outside bounds"));
    }
    Ok(Header {
        id: DeadlineId::from_uuid(row.try_get("deadline_id").map_err(inconsistent)?),
        case_id: CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?),
        revision,
        title: title_value,
        profile: DeadlineProfileRef {
            id: DeadlineProfileId::from_uuid(row.try_get("profile_id").map_err(inconsistent)?),
            revision: DeadlineProfileRevision::new(counter(
                row.try_get("profile_revision").map_err(inconsistent)?,
            )?)
            .map_err(inconsistent)?,
        },
        responsible: DeadlineResponsibleSnapshot {
            id: UserId::from_uuid(row.try_get("responsible_id").map_err(inconsistent)?),
            email: email(row.try_get("responsible_email").map_err(inconsistent)?)?,
            role: row
                .try_get::<_, &str>("responsible_role")
                .map_err(inconsistent)?
                .parse()
                .map_err(inconsistent)?,
        },
        attention: attention::decode(&row.try_get("attention").map_err(inconsistent)?)?,
        status: row
            .try_get::<_, &str>("status")
            .map_err(inconsistent)?
            .parse()
            .map_err(inconsistent)?,
        reason,
        receipt: DeadlineReceipt {
            operation_id: DeadlineOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            review_digest: digest(row.try_get("review_digest").map_err(inconsistent)?)?,
            capture_digest: digest(row.try_get("capture_digest").map_err(inconsistent)?)?,
            submission_digest: digest(row.try_get("submission_digest").map_err(inconsistent)?)?,
        },
        recorded_at,
        recorded_by: DeadlineActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email: email(row.try_get("recorded_by_email").map_err(inconsistent)?)?,
        },
    })
}
pub(super) fn command(value: &DeadlineDetail) -> Result<DeadlineCommand, ApplicationError> {
    let change = if value.receipt.action == DeadlineAction::Register {
        DeadlineChange::Register {
            definition: value.definition.clone(),
        }
    } else {
        let expected_revision =
            DeadlineRevision::new(value.receipt.expected_revision).map_err(inconsistent)?;
        let reason = value
            .reason
            .clone()
            .ok_or_else(|| inconsistent("deadline reason is absent"))?;
        match value.receipt.action {
            DeadlineAction::Correct => DeadlineChange::Correct {
                expected_revision,
                definition: value.definition.clone(),
                reason,
            },
            DeadlineAction::SetAttention => DeadlineChange::SetAttention {
                expected_revision,
                attention: value.attention.clone(),
                reason,
            },
            DeadlineAction::Retire => DeadlineChange::Retire {
                expected_revision,
                reason,
            },
            DeadlineAction::Register => unreachable!("register was handled"),
        }
    };
    Ok(DeadlineCommand {
        operation_id: value.receipt.operation_id,
        deadline_id: value.id,
        change,
    })
}

pub(super) fn projection(value: &DeadlineDetail) -> serde_json::Value {
    serde_json::json!({"actor_id":value.recorded_by.id.to_string(),"case_id":value.case_id.to_string(),
        "deadline_id":value.id.to_string(),"operation_id":value.receipt.operation_id.to_string(),
        "action":value.receipt.action.as_str(),"expected_revision":value.receipt.expected_revision,
        "review_digest":value.receipt.review_digest.to_hex(),"reason":value.reason.as_ref().map(|r|r.as_str())})
}
