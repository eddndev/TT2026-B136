use super::{inconsistent, port};
use application::{
    case_stages::StageSupportSnapshot, procedural_facts::*, procedural_resources::*,
    typed_participants::ParticipantDetail, ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

pub(super) fn material(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &ResourceValues,
    hasher: &dyn DocumentHasher,
) -> Result<
    (
        Option<Box<ResolutionSourceMaterial>>,
        Vec<ParticipantDetail>,
    ),
    ApplicationError,
> {
    let selection = FactSourceSelection::select_resolution(values.resolution());
    let resolution =
        crate::procedural_fact_postgres::sources::material(tx, case, &selection, hasher)?
            .resolution;
    let mut selected = values
        .appellants()
        .iter()
        .filter_map(|v| v.participant())
        .collect::<Vec<_>>();
    selected.sort_by_key(|r| (r.id.as_uuid(), r.revision.get()));
    let participants = selected
        .into_iter()
        .map(|r| crate::participant_postgres::storage::exact(tx, case, r.id, r.revision, hasher))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((resolution, participants))
}
pub(super) fn reconstruct(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &ResourceValues,
    supports: Vec<StageSupportSnapshot>,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceSources, ApplicationError> {
    let (resolution, participants) = material(tx, case, values, hasher)?;
    let resolution = resolve_fact_resolution(
        hasher,
        case,
        &FactSourceSelection::select_resolution(values.resolution()),
        resolution.as_deref(),
    )?
    .ok_or_else(|| inconsistent("resource exact resolution is absent"))?;
    let mut appellants = Vec::new();
    let mut selected = values
        .appellants()
        .iter()
        .filter_map(|v| v.participant())
        .collect::<Vec<_>>();
    selected.sort_by_key(|r| (r.id.as_uuid(), r.revision.get()));
    for (reference, detail) in selected.iter().zip(&participants) {
        appellants.extend(resolve_fact_participants(
            hasher,
            case,
            &FactSourceSelection::select_participant(*reference),
            std::slice::from_ref(detail),
        )?);
    }
    validate_supports(tx, case, &supports)?;
    Ok(ResourceSources {
        resolution,
        appellants,
        supports,
    })
}
pub(super) fn validate_supports(
    tx: &mut Transaction<'_>,
    case: CaseId,
    supports: &[StageSupportSnapshot],
) -> Result<(), ApplicationError> {
    for support in supports {
        let row=tx.query_opt("SELECT name,digest FROM documents WHERE case_id=$1 AND id=$2 AND version=$3 AND octet_length(name)<=128 AND octet_length(digest)=32",
            &[&case.as_uuid(),&support.reference.id.as_uuid(),&i64::from(support.reference.version.get())]).map_err(port)?
            .ok_or_else(||inconsistent("resource exact support is absent or unbounded"))?;
        if row.get::<_, String>(0) != support.name
            || row.get::<_, Vec<u8>>(1) != support.digest.as_bytes().as_slice()
        {
            return Err(inconsistent("resource retained support metadata differs"));
        }
    }
    Ok(())
}
