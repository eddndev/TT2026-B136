use super::inconsistent;
use application::{
    case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot},
    cases::CaseActorSnapshot,
    hearing_results::*,
    ApplicationError,
};
use domain::{
    case_administration::CaseRevision,
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    hearings::{HearingId, HearingRevision},
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
    hasher: &dyn DocumentHasher,
) -> Result<(HearingResultSnapshot, Option<StageSupportSnapshot>), ApplicationError> {
    if row
        .try_get::<_, i64>("initial_revision")
        .map_err(inconsistent)?
        != 1
    {
        return Err(inconsistent("result root has invalid first revision"));
    }
    let canonical: Vec<u8> = row.try_get("values_canonical").map_err(inconsistent)?;
    let values = crate::hearing_result_codec::values(
        &canonical,
        &row.try_get("values_view").map_err(inconsistent)?,
    )?;
    let action = match row.try_get::<_, &str>("action").map_err(inconsistent)? {
        "record" => HearingResultAction::Record,
        "correct" => HearingResultAction::Correct,
        "withdraw" => HearingResultAction::Withdraw,
        _ => return Err(inconsistent("invalid result action")),
    };
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|raw| {
            let result = HearingResultText::new(&raw).map_err(inconsistent)?;
            if result.as_str() != raw {
                return Err(inconsistent("noncanonical result reason"));
            }
            Ok(result)
        })
        .transpose()?;
    let hearing_id = HearingId::from_uuid(row.try_get("hearing_id").map_err(inconsistent)?);
    let anchor = HearingResultAnchor {
        hearing_id,
        revision: HearingRevision::new(counter(
            row.try_get("anchor_revision").map_err(inconsistent)?,
        )?)
        .map_err(inconsistent)?,
        values_digest: digest(row.try_get("anchor_values_digest").map_err(inconsistent)?)?,
        submission_digest: digest(
            row.try_get("anchor_submission_digest")
                .map_err(inconsistent)?,
        )?,
    };
    let revision =
        HearingResultRevision::new(counter(row.try_get("revision").map_err(inconsistent)?)?)
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
        return Err(inconsistent("invalid result capture time or author"));
    }
    let snapshot = HearingResultSnapshot {
        case_id: CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?),
        hearing_id,
        id: HearingResultId::from_uuid(row.try_get("result_id").map_err(inconsistent)?),
        revision,
        values,
        values_digest: digest(row.try_get("values_digest").map_err(inconsistent)?)?,
        status: row
            .try_get::<_, &str>("status")
            .map_err(inconsistent)?
            .parse()
            .map_err(inconsistent)?,
        reason,
        receipt: HearingResultReceipt {
            operation_id: HearingResultOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            submission_digest: digest(row.try_get("submission_digest").map_err(inconsistent)?)?,
        },
        anchor,
        continuation: continuation(row)?,
        recorded_administration_revision: CaseRevision::new(counter(
            row.try_get("recorded_administration_revision")
                .map_err(inconsistent)?,
        )?)
        .map_err(inconsistent)?,
        recorded_administration_digest: digest(
            row.try_get("recorded_administration_digest")
                .map_err(inconsistent)?,
        )?,
        recorded_at: at,
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email,
        },
    };
    hearing_result_snapshot_receipt_matches(hasher, &snapshot)?;
    let submission: Vec<u8> = row.try_get("submission_canonical").map_err(inconsistent)?;
    if hearing_result_submission_bytes(
        snapshot.recorded_by.id,
        snapshot.case_id,
        &command(&snapshot)?,
        &snapshot.anchor,
        snapshot.continuation.as_ref(),
        snapshot.values_digest,
    ) != submission
        || hasher.hash_bytes(&submission) != snapshot.receipt.submission_digest
        || row
            .try_get::<_, serde_json::Value>("submission_view")
            .map_err(inconsistent)?
            != receipt_projection(&snapshot)
    {
        return Err(inconsistent("result receipt bytes or projection differ"));
    }
    let support = support(row, &snapshot.values)?;
    Ok((snapshot, support))
}
fn continuation(row: &Row) -> Result<Option<HearingResultContinuation>, ApplicationError> {
    let fields = (
        row.try_get::<_, Option<Uuid>>("continuation_hearing_id")
            .map_err(inconsistent)?,
        row.try_get::<_, Option<Uuid>>("continuation_result_id")
            .map_err(inconsistent)?,
        row.try_get::<_, Option<i64>>("continuation_revision")
            .map_err(inconsistent)?,
        row.try_get::<_, Option<Vec<u8>>>("continuation_values_digest")
            .map_err(inconsistent)?,
        row.try_get::<_, Option<Vec<u8>>>("continuation_submission_digest")
            .map_err(inconsistent)?,
    );
    match fields {
        (None, None, None, None, None) => Ok(None),
        (Some(hearing), Some(id), Some(revision), Some(values), Some(submission)) => {
            Ok(Some(HearingResultContinuation {
                hearing_id: HearingId::from_uuid(hearing),
                result_id: HearingResultId::from_uuid(id),
                revision: HearingResultRevision::new(counter(revision)?).map_err(inconsistent)?,
                values_digest: digest(values)?,
                submission_digest: digest(submission)?,
            }))
        }
        _ => Err(inconsistent("incomplete result continuation")),
    }
}
pub(super) fn command(
    snapshot: &HearingResultSnapshot,
) -> Result<HearingResultCommand, ApplicationError> {
    let change = match snapshot.receipt.action {
        HearingResultAction::Record => HearingResultChange::Record {
            anchor_revision: snapshot.anchor.revision,
            continuation: snapshot
                .continuation
                .map(|c| HearingResultContinuationRef::new(c.result_id, c.revision)),
            values: snapshot.values.clone(),
        },
        HearingResultAction::Correct => HearingResultChange::Correct {
            expected_revision: HearingResultRevision::new(snapshot.receipt.expected_revision)
                .map_err(inconsistent)?,
            values: snapshot.values.clone(),
            reason: snapshot
                .reason
                .clone()
                .ok_or_else(|| inconsistent("correction reason missing"))?,
        },
        HearingResultAction::Withdraw => HearingResultChange::Withdraw {
            expected_revision: HearingResultRevision::new(snapshot.receipt.expected_revision)
                .map_err(inconsistent)?,
            reason: snapshot
                .reason
                .clone()
                .ok_or_else(|| inconsistent("withdrawal reason missing"))?,
        },
    };
    Ok(HearingResultCommand {
        operation_id: snapshot.receipt.operation_id,
        hearing_id: snapshot.hearing_id,
        result_id: snapshot.id,
        change,
    })
}
fn support(
    row: &Row,
    values: &HearingResultValues,
) -> Result<Option<StageSupportSnapshot>, ApplicationError> {
    let name = row
        .try_get::<_, Option<String>>("support_name")
        .map_err(inconsistent)?;
    let format = row
        .try_get::<_, Option<String>>("support_format")
        .map_err(inconsistent)?;
    let policy = row
        .try_get::<_, Option<String>>("support_policy")
        .map_err(inconsistent)?;
    match (values.provenance().support(), name, format, policy) {
        (None, None, None, None) => Ok(None),
        (Some(reference), Some(name), Some(format), Some(policy)) => {
            if name.is_empty() || name.len() > 128 || policy != "pdf_docx_v1" {
                return Err(inconsistent("invalid result support capture"));
            }
            let format = match format.as_str() {
                "pdf" => StageDocumentFormat::Pdf,
                "docx" => StageDocumentFormat::Docx,
                _ => return Err(inconsistent("invalid result support format")),
            };
            Ok(Some(StageSupportSnapshot {
                reference: reference.reference(),
                digest: reference.digest(),
                name,
                format,
                policy: StageFormatPolicy::PdfDocxV1,
            }))
        }
        _ => Err(inconsistent("incomplete result support capture")),
    }
}
fn receipt_projection(s: &HearingResultSnapshot) -> serde_json::Value {
    serde_json::json!({"operation_id":s.receipt.operation_id.to_string(),"actor_id":s.recorded_by.id.to_string(),
        "case_id":s.case_id.to_string(),"hearing_id":s.hearing_id.to_string(),"result_id":s.id.to_string(),
        "action":s.receipt.action.as_str(),"expected_revision":s.receipt.expected_revision,
        "values_digest":s.values_digest.to_hex(),"reason":s.reason.as_ref().map(HearingResultText::as_str),
        "anchor":{"revision":s.anchor.revision.get(),"values_digest":s.anchor.values_digest.to_hex(),"submission_digest":s.anchor.submission_digest.to_hex()},
        "continuation":s.continuation.map(|c|serde_json::json!({"hearing_id":c.hearing_id.to_string(),"result_id":c.result_id.to_string(),
            "revision":c.revision.get(),"values_digest":c.values_digest.to_hex(),"submission_digest":c.submission_digest.to_hex()}))})
}
