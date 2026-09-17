use super::{inconsistent, port, storage};
use application::{procedural_facts::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;
use std::collections::BTreeSet;

pub(super) fn material(
    tx: &mut Transaction<'_>,
    case: CaseId,
    selection: &FactSourceSelection,
    hasher: &dyn DocumentHasher,
) -> Result<FactSourceMaterial, ApplicationError> {
    let resolution = selection
        .resolution()
        .map(|reference| {
            let detail = storage::detail(
                tx,
                case,
                FactTarget::Resolution(reference.id),
                Some(reference.revision),
                hasher,
            )
            .map_err(missing)?;
            let ProceduralFactSnapshot::Resolution(snapshot) = detail.snapshot else {
                return Err(inconsistent("parent is not a resolution"));
            };
            let hearing = snapshot
                .values
                .provenance()
                .hearing_reference()
                .map(|r| hearing(tx, case, r, hasher))
                .transpose()?;
            Ok(Box::new(ResolutionSourceMaterial {
                snapshot: *snapshot,
                hearing,
                admitted_support: detail.sources.direct_supports.into_iter().next(),
            }))
        })
        .transpose()?;
    let participants = selection
        .participants()
        .iter()
        .map(|r| {
            crate::participant_postgres::storage::exact(tx, case, r.id, r.revision, hasher)
                .map_err(missing)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    let mut hearing_results = Vec::new();
    for r in selection.hearing_results() {
        if seen.insert((
            r.hearing_id.as_uuid(),
            r.result_id.as_uuid(),
            r.revision.get(),
        )) {
            hearing_results.push(hearing(tx, case, *r, hasher)?);
        }
    }
    Ok(FactSourceMaterial {
        resolution,
        participants,
        hearing_results,
    })
}
fn hearing(
    tx: &mut Transaction<'_>,
    case: CaseId,
    r: FactHearingRef,
    hasher: &dyn DocumentHasher,
) -> Result<application::hearing_results::HearingResultSnapshot, ApplicationError> {
    crate::hearing_result_postgres::storage::snapshot(
        tx,
        case,
        r.hearing_id,
        r.result_id,
        r.revision,
        hasher,
    )
    .map_err(missing)
}
pub(super) fn validate(
    tx: &mut Transaction<'_>,
    detail: &FactDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let case = detail.snapshot.case_id();
    let selection = FactSourceSelection::from_values(&detail.snapshot.values());
    let material = material(tx, case, &selection, hasher).map_err(|e| match e {
        ApplicationError::ProceduralFact(ProceduralFactError::ReferenceNotFound) => {
            inconsistent("exact historical fact source is absent")
        }
        other => other,
    })?;
    let captured = &detail.snapshot.metadata().recorded_administration;
    if let Some(parent) = &material.resolution {
        validate_fact_administration(
            hasher,
            case,
            captured,
            Some(&parent.snapshot.metadata.recorded_administration),
        )?;
    }
    for source in &material.hearing_results {
        let recorded = captured
            .snapshot()
            .ok_or_else(|| inconsistent("fact capture predates its exact result source"))?;
        if recorded.revision < source.recorded_administration_revision
            || (recorded.revision == source.recorded_administration_revision
                && recorded.values_digest != source.recorded_administration_digest)
        {
            return Err(inconsistent(
                "fact capture predates or contradicts its exact result source",
            ));
        }
    }
    let resolution =
        resolve_fact_resolution(hasher, case, &selection, material.resolution.as_deref())?;
    let participants = resolve_fact_participants(hasher, case, &selection, &material.participants)?;
    let hearings = resolve_fact_hearings(hasher, case, &selection, &material.hearing_results)?;
    let expected = FactSources {
        resolved: FactResolvedSources {
            resolution: resolution.as_ref().map(|r| r.snapshot),
            participants: participants.iter().map(|v| v.snapshot).collect(),
            hearing_results: hearings.iter().map(|v| v.snapshot).collect(),
        },
        views: FactSourceViews {
            resolution: resolution.map(|r| r.view),
            participants: participants.into_iter().map(|v| v.overview).collect(),
            hearing_results: hearings.into_iter().map(|v| v.view).collect(),
        },
        direct_supports: detail.sources.direct_supports.clone(),
    };
    if expected != detail.sources {
        return Err(inconsistent(
            "captured fact projections differ from exact history",
        ));
    }
    for support in &detail.sources.direct_supports {
        let row=tx.query_opt("SELECT name,digest FROM documents WHERE case_id=$1 AND id=$2 AND version=$3 AND octet_length(name)<=128 AND octet_length(digest)=32", &[&case.as_uuid(),&support.reference.id.as_uuid(),&i64::from(support.reference.version.get())]).map_err(port)?.ok_or_else(||inconsistent("exact fact support is absent or unbounded"))?;
        if row.get::<_, String>(0) != support.name
            || row.get::<_, Vec<u8>>(1) != support.digest.as_bytes().as_slice()
        {
            return Err(inconsistent("exact fact support differs"));
        }
    }
    Ok(())
}
pub(super) fn missing(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::ParticipantNotFound
        | ApplicationError::DocumentNotFound(_)
        | ApplicationError::HearingResult(
            application::hearing_results::HearingResultError::NotFound,
        )
        | ApplicationError::ProceduralFact(ProceduralFactError::NotFound) => {
            ProceduralFactError::ReferenceNotFound.into()
        }
        other => other,
    }
}
