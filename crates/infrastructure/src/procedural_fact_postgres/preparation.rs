use super::{port, sources, storage};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, procedural_facts::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &ProceduralFactCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<FactPreparation, ApplicationError> {
    command.result_revision()?;
    let used: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_procedural_fact_revisions WHERE operation_id=$1)",
            &[&command.operation_id().as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if used {
        return Err(ProceduralFactError::OperationConflict.into());
    }
    let base = match storage::detail(tx, case, command.target(), None, hasher) {
        Ok(value) => Some(value),
        Err(ApplicationError::ProceduralFact(ProceduralFactError::NotFound)) => None,
        Err(e) => return Err(e),
    };
    validate_fact_base(case, command, base.as_ref().map(|v| &v.snapshot))?;
    let observed_administration = crate::cases::storage::detail(tx, case, hasher)?.administration;
    validate_fact_administration(
        hasher,
        case,
        &observed_administration,
        base.as_ref()
            .map(|v| &v.snapshot.metadata().recorded_administration),
    )?;
    let (source_material, records) = if command.action() == FactAction::Withdraw {
        (
            FactSourceMaterial {
                resolution: None,
                participants: vec![],
                hearing_results: vec![],
            },
            vec![],
        )
    } else {
        let values = match command {
            ProceduralFactCommand::Resolution(c) => ProceduralFactValues::Resolution(Box::new(
                c.change()
                    .values()
                    .ok_or(ProceduralFactError::InvalidReference)?
                    .clone(),
            )),
            ProceduralFactCommand::Notification(c) => ProceduralFactValues::Notification(Box::new(
                c.change()
                    .values()
                    .ok_or(ProceduralFactError::InvalidReference)?
                    .clone(),
            )),
        };
        let selection = FactSourceSelection::from_values(&values);
        let material = sources::material(tx, case, &selection, hasher)?;
        let records = selection
            .direct_supports()
            .iter()
            .map(|s| {
                crate::case_stages::documents::load(
                    tx,
                    case,
                    StageSupportRef::new(s.reference(), s.digest()),
                    limits,
                )
                .map_err(sources::missing)
            })
            .collect::<Result<Vec<_>, _>>()?;
        (material, records)
    };
    Ok(FactPreparation {
        case_id: case,
        base,
        observed_administration,
        source_material,
        records,
    })
}
