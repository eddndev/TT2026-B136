use super::{anchors, decode::digest, inconsistent, sources};
use application::{
    case_stages::StageDocumentFormat,
    identity::Principal,
    precautionary_hearings::{PrecautionaryContext, PrecautionaryContextExpectation},
    precautionary_measures::*,
    ApplicationError,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::DocumentHasher,
    identity::UserId,
    precautionary_measures::*,
};
use postgres::{Row, Transaction};

pub(super) struct DecisionInput {
    pub case: CaseId,
    pub context: PrecautionaryContext,
    pub command: MeasureDecisionCommand,
    pub format: StageDocumentFormat,
    pub actor: Principal,
    pub at: time::OffsetDateTime,
}
fn revision(row: &Row, name: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.get::<_, i64>(name)).map_err(inconsistent)
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<DecisionInput, ApplicationError> {
    let case = CaseId::from_uuid(row.get("case_id"));
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
    Ok(DecisionInput {
        case,
        context,
        command,
        format,
        actor,
        at,
    })
}
