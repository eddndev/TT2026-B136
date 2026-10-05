use super::{dependencies, inconsistent, port};
use crate::measure_decision_postgres::{
    load_precautionary_history, HistoryReserve, HistoryRoot, LoadedMeasureHistory,
};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, measure_corrections::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_hearings::{MeasureRevision, PrecautionaryMeasureRef},
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureAdministrativeCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureAdministrativeReady, ApplicationError> {
    load_with_proof(tx, case, command, limits, hasher).map(|(ready, _)| ready)
}

pub(super) fn load_with_proof(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureAdministrativeCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<(MeasureAdministrativeReady, LoadedMeasureHistory), ApplicationError> {
    crate::measure_decision_postgres::audit::inventory_intact(tx)?;
    let context =
        crate::precautionary_hearing_postgres::sources::current_context(tx, case, hasher)?;
    if command.context.administration_revision != context.material().administration.revision
        || command.context.stage_revision != context.material().stage.stage_revision()
        || command.context.context_digest != context.digest(hasher)
    {
        return Err(MeasureAdministrativeError::SubmissionMismatch.into());
    }
    let row = tx
        .query_opt(
            "SELECT revision,validity,family,
         CASE WHEN octet_length(capture_digest)=32 THEN capture_digest END AS capture_digest
         FROM case_measure_revisions WHERE case_id=$1 AND measure_id=$2
         ORDER BY revision DESC LIMIT 1",
            &[&case.as_uuid(), &command.target.id().as_uuid()],
        )
        .map_err(port)?
        .ok_or(MeasureAdministrativeError::NotFound)?;
    let revision =
        MeasureRevision::new(u32::try_from(row.get::<_, i64>("revision")).map_err(inconsistent)?)
            .map_err(inconsistent)?;
    let digest = super::decode::digest(
        row.get::<_, Option<Vec<u8>>>("capture_digest")
            .ok_or_else(|| inconsistent("current record digest exceeds bounds"))?,
    )?;
    let target_head = PrecautionaryMeasureRef::new(command.target.id(), revision, digest);
    if target_head != command.target {
        return Err(MeasureAdministrativeError::StaleHead.into());
    }
    if !matches!(row.get::<_, String>("family").as_str(), "m1" | "m2" | "c1")
        || row.get::<_, String>("validity") != "valid"
    {
        return Err(inconsistent(
            "current target family or captured validity is ineligible",
        ));
    }
    let mut roots = dependencies::roots(tx, case, command.target, hasher)?;
    roots.push(HistoryRoot::Measure(command.target));
    let loaded = load_precautionary_history(
        tx,
        case,
        &roots,
        HistoryReserve {
            groups: 1,
            members: 1,
            ..HistoryReserve::default()
        },
        hasher,
    )?;
    let dependency_inventory = loaded.dependency_inventory()?;
    let checked = inspect_measure_administrative_dependencies(
        hasher,
        case,
        command.target,
        &dependency_inventory,
    )?;
    if !checked.dependants().is_empty() {
        return Err(MeasureAdministrativeError::KnownDependants.into());
    }
    let history = loaded.record_subclosure(&[command.target])?;
    let targets =
        resolve_measure_records_with_decision_history(hasher, case, &[command.target], &history)?;
    let target = targets
        .targets()
        .first()
        .ok_or_else(|| inconsistent("validated target is absent"))?;
    if target.validity() != MeasureCaptureValidity::Valid {
        return Err(inconsistent("captured target is entered in error"));
    }
    let support = target.support();
    let support_record = crate::case_stages::documents::load(
        tx,
        case,
        StageSupportRef::new(support.reference, support.digest),
        limits,
    )?;
    Ok((
        MeasureAdministrativeReady {
            context,
            support_record,
            target_head,
            dependency_inventory,
        },
        loaded,
    ))
}
