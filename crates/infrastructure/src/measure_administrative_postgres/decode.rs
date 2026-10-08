use super::inconsistent;
use application::{
    identity::Principal, measure_corrections::*,
    precautionary_hearings::PrecautionaryContextExpectation,
    precautionary_measures::MeasureDecisionRecordHistoryEvidence, ApplicationError,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    hearings::HearingNote,
    identity::UserId,
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
    precautionary_measures::MeasureCorrectionOperationId,
    typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef},
};
use postgres::{Row, Transaction};

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(inconsistent)
}
fn revision(row: &Row, name: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.get::<_, i64>(name)).map_err(inconsistent)
}
/// Decode only an exact dependency selector before graph ordering or source hashes.
pub(crate) fn target(row: &Row) -> Result<PrecautionaryMeasureRef, ApplicationError> {
    Ok(PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(row.get("target_measure_id")),
        MeasureRevision::new(revision(row, "target_revision")?).map_err(inconsistent)?,
        digest(row.get("target_capture_digest"))?,
    ))
}
pub(crate) fn command(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureAdministrativeCommand, ApplicationError> {
    let action = row.get::<_, String>("action");
    let bytes = row.get::<_, Option<Vec<u8>>>("correction_canonical");
    let view = row.get::<_, Option<serde_json::Value>>("correction_view");
    let commitment = row.get::<_, Option<Vec<u8>>>("correction_digest");
    let replacement = replacement(row)?;
    let action = match (action.as_str(), bytes, view, commitment, replacement) {
        ("correct", Some(bytes), Some(view), Some(commitment), None) => {
            if hasher.hash_bytes(&bytes) != digest(commitment)? {
                return Err(inconsistent("correction values digest differs"));
            }
            MeasureAdministrativeAction::Correct(crate::measure_correction_codec::values(
                &bytes, &view,
            )?)
        }
        ("entered_in_error", None, None, None, None) => {
            MeasureAdministrativeAction::MarkEnteredInError
        }
        ("replace_entered_in_error", None, None, None, Some((replacement_id, subject))) => {
            MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
                replacement_id,
                subject,
            }
        }
        _ => {
            return Err(inconsistent(
                "administrative action and values shape differ",
            ))
        }
    };
    let reason = row.get::<_, String>("reason");
    let parsed = HearingNote::new(&reason).map_err(inconsistent)?;
    if parsed.as_str() != reason {
        return Err(inconsistent("administrative reason is not canonical"));
    }
    Ok(MeasureAdministrativeCommand {
        operation_id: MeasureCorrectionOperationId::from_uuid(row.get("operation_id")),
        target: PrecautionaryMeasureRef::new(
            MeasureId::from_uuid(row.get("target_measure_id")),
            MeasureRevision::new(revision(row, "target_revision")?).map_err(inconsistent)?,
            digest(row.get("target_capture_digest"))?,
        ),
        context: PrecautionaryContextExpectation {
            administration_revision: CaseRevision::new(revision(
                row,
                "observed_administration_revision",
            )?)
            .map_err(inconsistent)?,
            stage_revision: CaseStageRevision::new(revision(row, "observed_stage_revision")?)
                .map_err(inconsistent)?,
            context_digest: digest(row.get("observed_context_digest"))?,
        },
        reason: parsed,
        action,
    })
}
pub(crate) fn capture(
    tx: &mut Transaction<'_>,
    row: &Row,
    history: MeasureDecisionRecordHistoryEvidence,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
    if row.get::<_, String>("family") != "a1" {
        return Err(inconsistent("unsupported administrative owner family"));
    }
    let case = CaseId::from_uuid(row.get("case_id"));
    let command = command(row, hasher)?;
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: row
            .get::<_, String>("recorded_by_role")
            .parse()
            .map_err(inconsistent)?,
    };
    let context = crate::precautionary_hearing_postgres::sources::exact_context(
        tx,
        case,
        command.context.administration_revision,
        command.context.stage_revision,
        hasher,
    )
    .map_err(inconsistent)?;
    let subject = super::preparation::replacement_subject(tx, case, &command, hasher)?;
    let checked =
        super::preparation::checked(hasher, &actor, case, command, context, subject, &history)
            .map_err(inconsistent)?;
    let at = time::OffsetDateTime::from_unix_timestamp(row.get("recorded_at_seconds"))
        .map_err(inconsistent)?
        .replace_nanosecond(
            u32::try_from(row.get::<_, i32>("recorded_at_nanoseconds")).map_err(inconsistent)?,
        )
        .map_err(inconsistent)?;
    let capture = checked.into_capture(hasher, at).map_err(inconsistent)?;
    let review = &capture.review;
    if review.submission_digest != digest(row.get("submission_digest"))?
        || review.review_digest != digest(row.get("review_digest"))?
        || capture.capture_digest != digest(row.get("capture_digest"))?
        || capture.capture_digest != digest(row.get("owner_digest"))?
        || review.support.format.as_str() != row.get::<_, String>("support_format")
        || review.support.policy.as_str() != row.get::<_, String>("support_policy")
    {
        return Err(inconsistent(
            "administrative capture differs from original commitments or support",
        ));
    }
    let origin = measure_administrative_origin_with_decision_history(hasher, &capture, &history)
        .map_err(inconsistent)?;
    Ok(MeasureAdministrativeStoredOperation {
        capture,
        origin,
        record_history: history,
    })
}

fn replacement(row: &Row) -> Result<Option<(MeasureId, SubjectRevisionRef)>, ApplicationError> {
    let tuple = (
        row.get::<_, Option<uuid::Uuid>>("replacement_measure_id"),
        row.get::<_, Option<uuid::Uuid>>("replacement_subject_id"),
        row.get::<_, Option<i64>>("replacement_subject_revision"),
        row.get::<_, Option<Vec<u8>>>("replacement_subject_values_digest"),
    );
    match tuple {
        (None, None, None, None) => Ok(None),
        (Some(id), Some(subject), Some(revision), Some(bytes)) => Ok(Some((
            MeasureId::from_uuid(id),
            SubjectRevisionRef {
                id: CaseSubjectId::from_uuid(subject),
                revision: SubjectRevision::new(u32::try_from(revision).map_err(inconsistent)?)
                    .map_err(inconsistent)?,
                values_digest: digest(bytes)?,
            },
        ))),
        _ => Err(inconsistent(
            "replacement selectors require one complete exact tuple",
        )),
    }
}
