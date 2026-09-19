use crate::{error::ApiError, procedural_facts::sources::project};
use application::{
    case_stages::StageSupportSnapshot, procedural_facts::*, procedural_resources::*,
};
use domain::{cases::CaseId, procedural_facts::FactSupportRef};
use serde_json::{json, Value};
fn empty() -> FactSources {
    FactSources {
        resolved: FactResolvedSources {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        views: FactSourceViews {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        direct_supports: vec![],
    }
}
pub(super) fn validate(
    case: CaseId,
    values: &ResourceValues,
    sources: &ResourceSources,
) -> Result<(), ApiError> {
    // Hash verification belongs to the application. This boundary binds each
    // readable capture to the exact selection without trusting a supplied hash.
    resource_sources_bytes(sources).map_err(|_| ApiError::internal())?;
    if sources.resolution.snapshot.case_id != case
        || sources.resolution.snapshot.reference != values.resolution()
    {
        return Err(ApiError::internal());
    }
    let mut refs = values
        .appellants()
        .iter()
        .filter_map(|v| v.participant())
        .collect::<Vec<_>>();
    refs.sort_by_key(|r| (r.id.as_uuid(), r.revision.get()));
    if refs.len() != sources.appellants.len()
        || refs
            .iter()
            .zip(&sources.appellants)
            .any(|(r, s)| s.snapshot.case_id != case || s.snapshot.reference != *r)
    {
        return Err(ApiError::internal());
    }
    validate_supports(&values.direct_supports(), &sources.supports)
}
pub(super) fn validate_supports(
    refs: &[FactSupportRef],
    supports: &[StageSupportSnapshot],
) -> Result<(), ApiError> {
    if refs.len() != supports.len()
        || refs
            .iter()
            .zip(supports)
            .any(|(r, s)| r.reference() != s.reference || r.digest() != s.digest)
    {
        return Err(ApiError::internal());
    }
    let mut component = empty();
    component.direct_supports = supports.to_vec();
    fact_sources_bytes(&component).map_err(|_| ApiError::internal())?;
    Ok(())
}
pub(super) fn supports(values: &[StageSupportSnapshot]) -> Result<Value, ApiError> {
    let mut component = empty();
    component.direct_supports = values.to_vec();
    Ok(project(&component)?["direct_supports"].take())
}
pub(super) fn sources(value: &ResourceSources) -> Result<Value, ApiError> {
    let mut component = empty();
    component.resolved.resolution = Some(value.resolution.snapshot);
    component.views.resolution = Some(value.resolution.view.clone());
    let resolution = project(&component)?["resolution"].take();
    let appellants = value
        .appellants
        .iter()
        .map(|p| {
            let mut component = empty();
            component.resolved.participants.push(p.snapshot);
            component.views.participants.push(p.overview.clone());
            Ok::<_, ApiError>(project(&component)?["participants"][0].take())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"resolution":resolution,"appellants":appellants,"supports":supports(&value.supports)?}),
    )
}
