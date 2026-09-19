use super::{attention, inconsistent, projection};
use application::{
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision},
    deadline_reevaluation::{decode_tracked_submission, TechnicalCause, TrackedSubmission},
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
        "reevaluate" => DeadlineAction::Reevaluate,
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
    let (submission, recorded_by) = author(row)?;
    let version = submission
        .as_ref()
        .map_or(DeadlineReceiptVersion::Legacy, |value| {
            DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
                observations_digest: value.observations_digest,
                predecessor: value.predecessor,
                cause: value.cause,
            })
        });
    let header = Header {
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
            version,
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
        recorded_by,
    };
    if let Some(value) = submission {
        if value.case_id != header.case_id
            || value.deadline_id != header.id
            || value.operation_id != header.receipt.operation_id
            || value.action as u8 != header.receipt.action.tag()
            || value.expected_revision != header.receipt.expected_revision
            || value.review_digest != header.receipt.review_digest
            || value.reason.as_deref() != header.reason.as_ref().map(|reason| reason.as_str())
        {
            return Err(inconsistent(
                "tracked submission differs from deadline columns",
            ));
        }
    } else if action == DeadlineAction::Reevaluate {
        return Err(inconsistent(
            "legacy receipt cannot record technical reevaluation",
        ));
    }
    let event = match &header.receipt.version {
        DeadlineReceiptVersion::Tracked(metadata) => match metadata.cause {
            Some(TechnicalCause::SourceEvent { event, .. }) => {
                Some(i64::try_from(event.sequence).map_err(inconsistent)?)
            }
            _ => None,
        },
        DeadlineReceiptVersion::Legacy => None,
    };
    if row
        .try_get::<_, Option<i64>>("cause_event_sequence")
        .map_err(inconsistent)?
        != event
    {
        return Err(inconsistent("deadline event sequence projection differs"));
    }
    Ok(header)
}

fn author(
    row: &Row,
) -> Result<(Option<TrackedSubmission>, DeadlineActorSnapshot), ApplicationError> {
    let bytes: &[u8] = row.try_get("submission_canonical").map_err(inconsistent)?;
    let id: Option<uuid::Uuid> = row.try_get("recorded_by").map_err(inconsistent)?;
    let captured_email: Option<String> = row.try_get("recorded_by_email").map_err(inconsistent)?;
    match bytes.get(..5) {
        Some(b"DLTX1") => Ok((
            None,
            DeadlineActorSnapshot::User {
                id: UserId::from_uuid(id.ok_or_else(|| inconsistent("legacy author is absent"))?),
                email: email(
                    captured_email.ok_or_else(|| inconsistent("legacy author email is absent"))?,
                )?,
            },
        )),
        Some(b"DLTX2") => {
            let submission = decode_tracked_submission(bytes).map_err(inconsistent)?;
            match &submission.author {
                DeadlineActorSnapshot::User {
                    id: author_id,
                    email,
                } => {
                    if id != Some(author_id.as_uuid())
                        || captured_email.as_deref() != Some(email.as_str())
                    {
                        return Err(inconsistent("tracked user author differs from columns"));
                    }
                }
                DeadlineActorSnapshot::Technical { .. } => {
                    if id.is_some() || captured_email.is_some() {
                        return Err(inconsistent("technical author has fabricated user columns"));
                    }
                }
            }
            let author = submission.author.clone();
            Ok((Some(submission), author))
        }
        _ => Err(inconsistent("unsupported deadline submission version")),
    }
}

pub(super) fn legacy_actor(
    author: &DeadlineActorSnapshot,
) -> Result<(UserId, &str), ApplicationError> {
    match author {
        DeadlineActorSnapshot::User { id, email } => Ok((*id, email.as_str())),
        DeadlineActorSnapshot::Technical { .. } => Err(inconsistent(
            "legacy storage requires a human deadline author",
        )),
    }
}

pub(super) fn projection(value: &DeadlineDetail) -> Result<serde_json::Value, ApplicationError> {
    projection::submission(value)
}
