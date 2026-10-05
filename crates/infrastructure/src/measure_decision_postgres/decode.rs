use super::{anchors, history, inconsistent, sources};
use application::{
    case_stages::StageDocumentFormat, identity::Principal,
    precautionary_hearings::PrecautionaryContextExpectation, precautionary_measures::*,
    ApplicationError,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    precautionary_measures::*,
};
use postgres::{Row, Transaction};

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(inconsistent)
}
fn revision(row: &Row, name: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.get::<_, i64>(name)).map_err(inconsistent)
}
pub(super) fn group(
    tx: &mut Transaction<'_>,
    row: &Row,
    measure_history: MeasureHistoryEvidence,
    resolved_anchor: Option<MeasureDecisionAnchorMaterial>,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
    let case = CaseId::from_uuid(row.get("case_id"));
    if row.get::<_, String>("family") != "g1" {
        return Err(inconsistent("unsupported owner family"));
    }
    let anchor = anchors::reference(row)?;
    let context = sources::exact_context(
        tx,
        case,
        CaseRevision::new(revision(row, "observed_administration_revision")?)
            .map_err(inconsistent)?,
        CaseStageRevision::new(revision(row, "observed_stage_revision")?).map_err(inconsistent)?,
        hasher,
    )
    .map_err(inconsistent)?;
    let expectation = PrecautionaryContextExpectation {
        administration_revision: context.material().administration.revision,
        stage_revision: context.material().stage.stage_revision(),
        context_digest: context.digest(hasher),
    };
    if expectation.context_digest != digest(row.get("observed_context_digest"))? {
        return Err(inconsistent("context digest differs"));
    }
    let bytes: Vec<u8> = row.get("values_canonical");
    let outcome_bytes: Vec<u8> = row.get("outcome_canonical");
    let values = crate::measure_decision_codec::decision_values(&bytes, &row.get("values_view"))?;
    let outcome = crate::measure_decision_codec::outcome(&outcome_bytes, &row.get("outcome_view"))?;
    if hasher.hash_bytes(&bytes) != digest(row.get("values_digest"))?
        || hasher.hash_bytes(&outcome_bytes) != digest(row.get("outcome_digest"))?
    {
        return Err(inconsistent("instruction values digest differs"));
    }
    let command = MeasureDecisionCommand {
        operation_id: MeasureDecisionOperationId::from_uuid(row.get("operation_id")),
        decision_id: MeasureDecisionId::from_uuid(row.get("decision_id")),
        context: expectation,
        values,
        anchor,
        outcome,
    };
    let format = match row.get::<_, String>("support_format").as_str() {
        "pdf" => StageDocumentFormat::Pdf,
        "docx" => StageDocumentFormat::Docx,
        _ => return Err(inconsistent("invalid admission format")),
    };
    if row.get::<_, String>("support_policy") != "pdf_docx_v1" {
        return Err(inconsistent("invalid admission policy"));
    }
    let predecessors =
        history::predecessors(&history::selections(&command.outcome), &measure_history)?;
    let result_sources = sources::result_sources(tx, case, &command, &predecessors, hasher)?;
    let material = MeasureDecisionMaterial {
        context,
        support: sources::support(tx, case, command.values.support(), format)?,
        anchor: resolved_anchor,
        predecessors,
        result_sources,
    };
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: row
            .get::<_, String>("recorded_by_role")
            .parse()
            .map_err(inconsistent)?,
    };
    let at = time::OffsetDateTime::from_unix_timestamp(row.get("recorded_at_seconds"))
        .map_err(inconsistent)?
        .replace_nanosecond(
            u32::try_from(row.get::<_, i32>("recorded_at_nanoseconds")).map_err(inconsistent)?,
        )
        .map_err(inconsistent)?;
    let group = prepare_measure_decision_with_history(
        hasher,
        &actor,
        case,
        command,
        material,
        &measure_history,
    )
    .map_err(inconsistent)?
    .into_group_capture(hasher, at)
    .map_err(inconsistent)?;
    if group.review.submission_digest != digest(row.get("submission_digest"))?
        || group.review.review_digest != digest(row.get("review_digest"))?
        || group.decision.capture_digest != digest(row.get("decision_digest"))?
        || group.capture_digest != digest(row.get("group_digest"))?
        || group.capture_digest != digest(row.get("owner_digest"))?
    {
        return Err(inconsistent("original group commitments differ"));
    }
    let origin = measure_group_origin(hasher, &group, &measure_history).map_err(inconsistent)?;
    Ok(MeasureDecisionStoredOperation {
        group,
        origin,
        measure_history,
    })
}

pub(super) fn member(
    row: &Row,
    capture: &MeasureCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let result = &capture.result;
    if row.get::<_, uuid::Uuid>("measure_id") != result.id.as_uuid()
        || row.get::<_, i64>("revision") != i64::from(result.revision.get())
        || row.get::<_, uuid::Uuid>("case_id") != capture.case_id.as_uuid()
        || row.get::<_, uuid::Uuid>("owner_operation") != capture.operation_id.as_uuid()
        || row.get::<_, String>("family") != "m1"
        || row.get::<_, String>("action") != action_name(result.action)
    {
        return Err(inconsistent("member identity or owner differs"));
    }
    let bytes: Vec<u8> = row.get("values_canonical");
    let values = crate::measure_decision_codec::measure_values(&bytes, &row.get("values_view"))?;
    if values != result.values
        || hasher.hash_bytes(&bytes) != digest(row.get("values_digest"))?
        || capture.capture_digest != digest(row.get("capture_digest"))?
    {
        return Err(inconsistent("member values or commitment differ"));
    }
    let subject = values.subject();
    if row.get::<_, uuid::Uuid>("subject_id") != subject.id.as_uuid()
        || row.get::<_, i64>("subject_revision") != i64::from(subject.revision.get())
        || digest(row.get("subject_values_digest"))? != subject.values_digest
    {
        return Err(inconsistent("member subject selectors differ"));
    }
    let pair = (
        row.get::<_, Option<uuid::Uuid>>("supervisor_id"),
        row.get::<_, Option<i64>>("supervisor_revision"),
    );
    let expected = match values.supervision() {
        MeasureSupervision::Unknown { .. } => (None, None),
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(i64::from(participant.revision().get())),
        ),
    };
    if pair != expected {
        return Err(inconsistent("member supervisor selectors differ"));
    }
    if row.get::<_, i64>("initial_revision") != 1
        || row.get::<_, uuid::Uuid>("root_operation") != result.origin.operation_id.as_uuid()
    {
        return Err(inconsistent(
            "measure root does not belong to original member",
        ));
    }
    Ok(())
}

pub(super) fn action_name(action: MeasureCaptureAction) -> &'static str {
    match action {
        MeasureCaptureAction::Impose => "impose",
        MeasureCaptureAction::Confirm => "confirm",
        MeasureCaptureAction::Modify => "modify",
        MeasureCaptureAction::Revoke => "revoke",
        MeasureCaptureAction::Cease => "cease",
        MeasureCaptureAction::SubstituteOut => "substitute_out",
        MeasureCaptureAction::SubstituteIn => "substitute_in",
    }
}
