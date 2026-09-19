use super::*;
use crate::{
    case_stages::StageSupportSnapshot,
    documents::{StageDocumentFormat, StageFormatPolicy},
    procedural_facts::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, DocumentVersionRef},
};

pub(super) fn resolve(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    values: &ResourceValues,
    material: &ResourceMaterial,
) -> Result<ResourceSources, ApplicationError> {
    let resolution = resolve_fact_resolution(
        hasher,
        case_id,
        &FactSourceSelection::select_resolution(values.resolution()),
        material.resolution.as_deref(),
    )?
    .ok_or_else(|| inconsistent("resource resolution material is absent"))?;
    let mut selected = values
        .appellants()
        .iter()
        .filter_map(|a| a.participant())
        .collect::<Vec<_>>();
    selected.sort_by_key(|r| (r.id.as_uuid(), r.revision.get()));
    if material.appellants.len() != selected.len() || selected.len() > 32 {
        return Err(inconsistent(
            "resource appellant material cardinality differs",
        ));
    }
    let mut materials = material.appellants.iter().collect::<Vec<_>>();
    materials.sort_by_key(|p| (p.id().as_uuid(), p.revision_number().get()));
    let appellants = selected
        .into_iter()
        .zip(materials)
        .map(|(reference, detail)| {
            let mut result = resolve_fact_participants(
                hasher,
                case_id,
                &FactSourceSelection::select_participant(reference),
                std::slice::from_ref(detail),
            )?;
            result
                .pop()
                .ok_or_else(|| inconsistent("verified appellant projection is absent"))
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(ResourceSources {
        resolution,
        appellants,
        supports: vec![],
    })
}
pub(super) fn check_records(
    material: &mut ResourceMaterial,
    selected: &[FactSupportRef],
) -> Result<(), ApplicationError> {
    if selected.len() > 2 || material.records.len() != selected.len() {
        return Err(inconsistent(
            "resource direct admission batch differs from selection",
        ));
    }
    material
        .records
        .sort_by_key(|r| (r.id.as_uuid(), r.version.get()));
    let mut selected = selected.to_vec();
    selected.sort_by_key(|r| (r.reference().id.as_uuid(), r.reference().version.get()));
    for (record, reference) in material.records.iter().zip(selected) {
        if record.id != reference.reference().id || record.version != reference.reference().version
        {
            return Err(inconsistent("resource direct document identity differs"));
        }
        if record.digest != reference.digest() {
            return Err(ApplicationError::StageSupportDigestMismatch);
        }
    }
    Ok(())
}
pub(super) fn admitted(
    material: &ResourceMaterial,
    formats: Vec<StageDocumentFormat>,
) -> Result<Vec<StageSupportSnapshot>, ApplicationError> {
    if formats.len() != material.records.len() {
        return Err(inconsistent(
            "resource admission result cardinality differs",
        ));
    }
    Ok(material
        .records
        .iter()
        .zip(formats)
        .map(|(record, format)| StageSupportSnapshot {
            reference: DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            digest: record.digest,
            name: record.name.clone(),
            format,
            policy: StageFormatPolicy::PdfDocxV1,
        })
        .collect())
}
pub(super) fn validate_selection(
    case_id: CaseId,
    values: &ResourceValues,
    sources: &ResourceSources,
) -> Result<(), ApplicationError> {
    if sources.resolution.snapshot.case_id != case_id
        || sources.resolution.snapshot.reference != values.resolution()
    {
        return Err(inconsistent(
            "resource resolution capture differs from selection",
        ));
    }
    let mut selected = values
        .appellants()
        .iter()
        .filter_map(|a| a.participant())
        .collect::<Vec<_>>();
    selected.sort_by_key(|r| (r.id.as_uuid(), r.revision.get()));
    if selected.len() != sources.appellants.len() || selected.len() > 32 {
        return Err(inconsistent("resource appellant capture count differs"));
    }
    for (reference, capture) in selected.iter().zip(&sources.appellants) {
        if capture.snapshot.case_id != case_id || capture.snapshot.reference != *reference {
            return Err(inconsistent(
                "resource appellant capture differs from exact selection",
            ));
        }
    }
    validate_supports(&values.direct_supports(), &sources.supports)?;
    resource_sources_bytes(sources)?;
    Ok(())
}
pub(super) fn validate_supports(
    selected: &[FactSupportRef],
    supports: &[StageSupportSnapshot],
) -> Result<(), ApplicationError> {
    let mut selected = selected.to_vec();
    selected.sort_by_key(|r| (r.reference().id.as_uuid(), r.reference().version.get()));
    if selected.len() > 2 || selected.len() != supports.len() {
        return Err(inconsistent(
            "resource support captures differ from direct selection",
        ));
    }
    for (reference, support) in selected.iter().zip(supports) {
        if reference.reference() != support.reference || reference.digest() != support.digest {
            return Err(inconsistent(
                "resource support capture differs from exact version",
            ));
        }
    }
    super::canonical::support_bytes(supports)?;
    Ok(())
}
pub(super) fn retained(
    base: &ResourceSources,
    current: &ResourceSources,
) -> Result<(), ApplicationError> {
    if base.resolution.snapshot.reference == current.resolution.snapshot.reference
        && base.resolution != current.resolution
    {
        return Err(inconsistent("retained resource resolution changed"));
    }
    for capture in &current.appellants {
        if base
            .appellants
            .iter()
            .any(|old| old.snapshot.reference == capture.snapshot.reference && old != capture)
        {
            return Err(inconsistent("retained resource appellant changed"));
        }
    }
    retained_supports(&base.supports, &current.supports)
}
pub(super) fn retained_supports(
    base: &[StageSupportSnapshot],
    current: &[StageSupportSnapshot],
) -> Result<(), ApplicationError> {
    for capture in current {
        if base
            .iter()
            .any(|old| old.reference == capture.reference && old != capture)
        {
            return Err(inconsistent("retained resource support changed"));
        }
    }
    Ok(())
}
