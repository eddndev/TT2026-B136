use super::{anchors, decode, inconsistent};
use application::{
    identity::Principal,
    precautionary_hearings::PrecautionaryContextExpectation,
    precautionary_measures::{measure_decision_submission_bytes, MeasureDecisionCommand},
    ApplicationError,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::DocumentHasher,
    identity::UserId,
    precautionary_measures::*,
};
use postgres::Row;

/// Bind advertised members to the submission retained by the original audit marker.
pub(crate) fn outcome(row: &Row) -> Result<MeasureDecisionOutcome, ApplicationError> {
    if !matches!(row.get::<_, String>("family").as_str(), "g1" | "g2")
        || decode::digest(row.get("owner_digest"))? != decode::digest(row.get("group_digest"))?
    {
        return Err(inconsistent(
            "advertised judicial owner family or digest differs",
        ));
    }
    let anchor = anchors::reference(row)?;
    let case = CaseId::from_uuid(row.get("case_id"));
    let revision = |name| u32::try_from(row.get::<_, i64>(name)).map_err(inconsistent);
    let values_bytes: Vec<u8> = row.get("values_canonical");
    let outcome_bytes: Vec<u8> = row.get("outcome_canonical");
    let hasher = crate::RingSha256Hasher;
    if hasher.hash_bytes(&values_bytes) != decode::digest(row.get("values_digest"))?
        || hasher.hash_bytes(&outcome_bytes) != decode::digest(row.get("outcome_digest"))?
    {
        return Err(inconsistent("advertised instruction digests differ"));
    }
    let command = MeasureDecisionCommand {
        operation_id: MeasureDecisionOperationId::from_uuid(row.get("operation_id")),
        decision_id: MeasureDecisionId::from_uuid(row.get("decision_id")),
        context: PrecautionaryContextExpectation {
            administration_revision: CaseRevision::new(revision(
                "observed_administration_revision",
            )?)
            .map_err(inconsistent)?,
            stage_revision: CaseStageRevision::new(revision("observed_stage_revision")?)
                .map_err(inconsistent)?,
            context_digest: decode::digest(row.get("observed_context_digest"))?,
        },
        values: crate::measure_decision_codec::decision_values(
            &values_bytes,
            &row.get("values_view"),
        )?,
        anchor,
        outcome: crate::measure_decision_codec::outcome(&outcome_bytes, &row.get("outcome_view"))?,
    };
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: row
            .get::<_, String>("recorded_by_role")
            .parse()
            .map_err(inconsistent)?,
    };
    if hasher.hash_bytes(&measure_decision_submission_bytes(&actor, case, &command)?)
        != decode::digest(row.get("submission_digest"))?
    {
        return Err(inconsistent(
            "advertised outcome differs from original submission",
        ));
    }
    Ok(command.outcome)
}
