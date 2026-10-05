use super::{inconsistent, sources};
use application::{
    case_stages::StageDocumentFormat, identity::Principal, precautionary_hearings::*,
    precautionary_measures::MeasureHistoryEvidence, ApplicationError,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    hearings::HearingNote,
    identity::UserId,
    precautionary_hearings::*,
};
use postgres::{Row, Transaction};

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Ok(Sha256Digest::from_array(
        bytes
            .try_into()
            .map_err(|_| inconsistent("invalid digest size"))?,
    ))
}
fn revision(row: &Row, name: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.get::<_, i64>(name)).map_err(inconsistent)
}
fn reason(row: &Row) -> Result<HearingNote, ApplicationError> {
    let text: String = row
        .get::<_, Option<String>>("reason")
        .ok_or_else(|| inconsistent("missing change reason"))?;
    let value = HearingNote::new(&text).map_err(inconsistent)?;
    if value.as_str() != text {
        return Err(inconsistent("noncanonical change reason"));
    }
    Ok(value)
}
pub(super) fn capture(
    tx: &mut Transaction<'_>,
    row: &Row,
    previous: Option<&PrecautionaryHearingCapture>,
    measure_history: &MeasureHistoryEvidence,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingCapture, ApplicationError> {
    let case = CaseId::from_uuid(row.get("case_id"));
    let context = sources::exact_context(
        tx,
        case,
        CaseRevision::new(revision(row, "observed_administration_revision")?)
            .map_err(inconsistent)?,
        CaseStageRevision::new(revision(row, "observed_stage_revision")?).map_err(inconsistent)?,
        hasher,
    )?;
    let expectation = PrecautionaryContextExpectation {
        administration_revision: context.material().administration.revision,
        stage_revision: context.material().stage.stage_revision(),
        context_digest: context.digest(hasher),
    };
    if expectation.context_digest != digest(row.get("observed_context_digest"))? {
        return Err(inconsistent("stored context digest differs"));
    }
    let action: String = row.get("action");
    let result =
        PrecautionaryHearingRevision::new(revision(row, "revision")?).map_err(inconsistent)?;
    let old: Option<Vec<u8>> = row.get("previous_capture_digest");
    let (change, sources) = if action == "cancel" {
        for name in ["values_canonical", "values_digest"] {
            if row.get::<_, Option<Vec<u8>>>(name).is_some() {
                return Err(inconsistent("cancellation has selected values"));
            }
        }
        if row
            .get::<_, Option<serde_json::Value>>("values_view")
            .is_some()
            || row.get::<_, Option<String>>("support_format").is_some()
            || row.get::<_, Option<String>>("support_policy").is_some()
        {
            return Err(inconsistent("cancellation has selected admission"));
        }
        let prior = previous.ok_or_else(|| inconsistent("cancellation has no predecessor"))?;
        (
            PrecautionaryHearingChange::Cancel {
                expected_revision: prior.review.result_revision,
                expected_capture_digest: digest(
                    old.ok_or_else(|| inconsistent("missing predecessor digest"))?,
                )?,
                reason: reason(row)?,
            },
            prior.review.sources.clone(),
        )
    } else {
        let canonical: Vec<u8> = row
            .get::<_, Option<Vec<u8>>>("values_canonical")
            .ok_or_else(|| inconsistent("missing selected values"))?;
        let projection: serde_json::Value = row
            .get::<_, Option<serde_json::Value>>("values_view")
            .ok_or_else(|| inconsistent("missing selected projection"))?;
        let values = crate::precautionary_hearing_codec::values(&canonical, &projection)?;
        if hasher.hash_bytes(&canonical)
            != digest(
                row.get::<_, Option<Vec<u8>>>("values_digest")
                    .ok_or_else(|| inconsistent("missing values digest"))?,
            )?
        {
            return Err(inconsistent("values digest differs"));
        }
        let format = match row.get::<_, Option<String>>("support_format").as_deref() {
            Some("pdf") => StageDocumentFormat::Pdf,
            Some("docx") => StageDocumentFormat::Docx,
            _ => return Err(inconsistent("invalid support format")),
        };
        if row.get::<_, Option<String>>("support_policy").as_deref() != Some("pdf_docx_v1") {
            return Err(inconsistent("invalid support admission policy"));
        }
        let sources = PrecautionaryHearingSources {
            participants: sources::participants(tx, case, &values, previous, false, hasher)?,
            support: sources::support(tx, case, &values, format)?,
        };
        let change = match action.as_str() {
            "schedule"
                if previous.is_none()
                    && old.is_none()
                    && row.get::<_, Option<String>>("reason").is_none() =>
            {
                PrecautionaryHearingChange::Schedule {
                    context: expectation,
                    values,
                }
            }
            "replace" => PrecautionaryHearingChange::Replace {
                expected_revision: previous
                    .ok_or_else(|| inconsistent("replacement has no predecessor"))?
                    .review
                    .result_revision,
                expected_capture_digest: digest(
                    old.ok_or_else(|| inconsistent("missing predecessor digest"))?,
                )?,
                context: expectation,
                values,
                reason: reason(row)?,
            },
            _ => return Err(inconsistent("invalid hearing action or predecessor")),
        };
        (change, sources)
    };
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: row
            .get::<_, String>("recorded_by_role")
            .parse()
            .map_err(inconsistent)?,
    };
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::from_uuid(row.get("operation_id")),
        hearing_id: PrecautionaryHearingId::from_uuid(row.get("hearing_id")),
        change,
    };
    if command.result_revision()? != result {
        return Err(inconsistent("revision differs from original command"));
    }
    let nanos =
        u32::try_from(row.get::<_, i32>("recorded_at_nanoseconds")).map_err(inconsistent)?;
    let at = time::OffsetDateTime::from_unix_timestamp(row.get("recorded_at_seconds"))
        .map_err(inconsistent)?
        .replace_nanosecond(nanos)
        .map_err(inconsistent)?;
    let capture = prepare_precautionary_hearing_with_history(
        hasher,
        &actor,
        case,
        command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: context,
            sources,
            predecessor: previous,
            measure_history,
        },
    )
    .map_err(inconsistent)?
    .into_capture(hasher, at)
    .map_err(inconsistent)?;
    if capture.review.submission_digest != digest(row.get("submission_digest"))?
        || capture.review.review_digest != digest(row.get("review_digest"))?
        || capture.capture_digest != digest(row.get("capture_digest"))?
    {
        return Err(inconsistent("stored receipt commitments differ"));
    }
    Ok(capture)
}
