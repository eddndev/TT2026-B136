use super::inconsistent;
use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};
use application::cases::CaseActorSnapshot;
use application::{hearings::*, ApplicationError};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
use postgres::Row;
use time::OffsetDateTime;

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(inconsistent)
}
pub(super) fn counter(value: i64) -> Result<u32, ApplicationError> {
    u32::try_from(value).map_err(inconsistent)
}
pub(super) fn row(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<HearingDetail, ApplicationError> {
    let canonical: Vec<u8> = row.try_get("values_canonical").map_err(inconsistent)?;
    let values = crate::hearing_codec::values(
        &canonical,
        &row.try_get("values_view").map_err(inconsistent)?,
    )?;
    let values_digest = digest(row.try_get("values_digest").map_err(inconsistent)?)?;
    if hearing_values_digest(hasher, &values) != values_digest {
        return Err(inconsistent("hearing value digest differs"));
    }
    let revision = HearingRevision::new(counter(row.try_get("revision").map_err(inconsistent)?)?)
        .map_err(inconsistent)?;
    let action = match row.try_get::<_, &str>("action").map_err(inconsistent)? {
        "schedule" => HearingAction::Schedule,
        "replace" => HearingAction::Replace,
        "cancel" => HearingAction::Cancel,
        _ => return Err(inconsistent("invalid hearing action")),
    };
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|raw| {
            let reason = HearingNote::new(&raw).map_err(inconsistent)?;
            if reason.as_str() != raw {
                return Err(inconsistent("noncanonical hearing reason"));
            }
            Ok(reason)
        })
        .transpose()?;
    let scheduling_context = HearingSchedulingContext {
        administration_revision: CaseRevision::new(counter(
            row.try_get("scheduling_administration_revision")
                .map_err(inconsistent)?,
        )?)
        .map_err(inconsistent)?,
        administration_digest: digest(
            row.try_get("scheduling_administration_digest")
                .map_err(inconsistent)?,
        )?,
        stage_revision: CaseStageRevision::new(counter(
            row.try_get("scheduling_stage_revision")
                .map_err(inconsistent)?,
        )?)
        .map_err(inconsistent)?,
        stage: row
            .try_get::<_, &str>("scheduling_stage")
            .map_err(inconsistent)?
            .parse()
            .map_err(inconsistent)?,
        stage_digest: row
            .try_get::<_, Option<Vec<u8>>>("scheduling_stage_digest")
            .map_err(inconsistent)?
            .map(digest)
            .transpose()?,
    };
    let email: String = row.try_get("recorded_by_email").map_err(inconsistent)?;
    if email.is_empty()
        || email.len() > 1280
        || email.trim() != email
        || email.chars().any(char::is_control)
    {
        return Err(inconsistent("noncanonical hearing actor email"));
    }
    let seconds: i64 = row.try_get("recorded_at_seconds").map_err(inconsistent)?;
    let nanos = counter(
        row.try_get::<_, i32>("recorded_at_nanoseconds")
            .map_err(inconsistent)?
            .into(),
    )?;
    let recorded_at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(inconsistent)?
        .replace_nanosecond(nanos)
        .map_err(inconsistent)?;
    if !(1..=9999).contains(&recorded_at.year()) {
        return Err(inconsistent("hearing capture year is invalid"));
    }
    let receipt = HearingReceipt {
        operation_id: HearingOperationId::from_uuid(
            row.try_get("operation_id").map_err(inconsistent)?,
        ),
        action,
        expected_revision: revision.get() - 1,
        expected_context: if action == HearingAction::Cancel {
            None
        } else {
            Some(HearingContextExpectation {
                case_revision: scheduling_context.administration_revision,
                stage_revision: scheduling_context.stage_revision,
            })
        },
        submission_digest: digest(row.try_get("submission_digest").map_err(inconsistent)?)?,
    };
    let snapshot = HearingSnapshot {
        case_id: CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?),
        id: HearingId::from_uuid(row.try_get("hearing_id").map_err(inconsistent)?),
        revision,
        values,
        values_digest,
        status: row
            .try_get::<_, &str>("status")
            .map_err(inconsistent)?
            .parse()
            .map_err(inconsistent)?,
        reason,
        receipt,
        scheduling_context,
        recorded_administration_revision: CaseRevision::new(counter(
            row.try_get("recorded_administration_revision")
                .map_err(inconsistent)?,
        )?)
        .map_err(inconsistent)?,
        recorded_administration_digest: digest(
            row.try_get("recorded_administration_digest")
                .map_err(inconsistent)?,
        )?,
        recorded_at,
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email,
        },
    };
    let canonical: Vec<u8> = row.try_get("submission_canonical").map_err(inconsistent)?;
    if hasher.hash_bytes(&canonical) != snapshot.receipt.submission_digest {
        return Err(inconsistent("hearing submission digest differs"));
    }
    let command = command(&snapshot)?;
    if hearing_submission_bytes(
        snapshot.recorded_by.id,
        snapshot.case_id,
        &command,
        snapshot.values_digest,
    ) != canonical
    {
        return Err(inconsistent(
            "hearing submission bytes differ from the captured command",
        ));
    }
    let support = support(row, &snapshot.values)?;
    Ok(HearingDetail {
        snapshot,
        participants: Vec::new(),
        support,
    })
}
pub(super) fn command(snapshot: &HearingSnapshot) -> Result<HearingCommand, ApplicationError> {
    let receipt = &snapshot.receipt;
    let change = match receipt.action {
        HearingAction::Schedule => HearingChange::Schedule {
            context: receipt
                .expected_context
                .ok_or_else(|| inconsistent("schedule context absent"))?,
            values: snapshot.values.clone(),
        },
        HearingAction::Replace => HearingChange::Replace {
            expected_revision: HearingRevision::new(receipt.expected_revision)
                .map_err(inconsistent)?,
            context: receipt
                .expected_context
                .ok_or_else(|| inconsistent("replace context absent"))?,
            values: snapshot.values.clone(),
            reason: snapshot
                .reason
                .clone()
                .ok_or_else(|| inconsistent("replace reason absent"))?,
        },
        HearingAction::Cancel => HearingChange::Cancel {
            expected_revision: HearingRevision::new(receipt.expected_revision)
                .map_err(inconsistent)?,
            reason: snapshot
                .reason
                .clone()
                .ok_or_else(|| inconsistent("cancel reason absent"))?,
        },
    };
    Ok(HearingCommand {
        operation_id: receipt.operation_id,
        hearing_id: snapshot.id,
        change,
    })
}
fn support(
    row: &Row,
    values: &HearingValues,
) -> Result<Option<StageSupportSnapshot>, ApplicationError> {
    let name: Option<String> = row.try_get("support_name").map_err(inconsistent)?;
    let format: Option<String> = row.try_get("support_format").map_err(inconsistent)?;
    let policy: Option<String> = row.try_get("support_policy").map_err(inconsistent)?;
    match (values.conviction_basis(), name, format, policy) {
        (None, None, None, None) => Ok(None),
        (Some(basis), Some(name), Some(format), Some(policy)) => {
            if name.is_empty() || name.len() > 128 || policy != "pdf_docx_v1" {
                return Err(inconsistent("invalid hearing support capture"));
            }
            let format = match format.as_str() {
                "pdf" => StageDocumentFormat::Pdf,
                "docx" => StageDocumentFormat::Docx,
                _ => return Err(inconsistent("invalid hearing support format")),
            };
            Ok(Some(StageSupportSnapshot {
                reference: basis.support().reference(),
                digest: basis.support().digest(),
                name,
                format,
                policy: StageFormatPolicy::PdfDocxV1,
            }))
        }
        _ => Err(inconsistent("hearing support capture is incomplete")),
    }
}
