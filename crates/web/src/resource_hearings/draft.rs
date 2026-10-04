use crate::{
    error::ApiError, procedural_facts::sources::project as project_sources,
    procedural_resources as resources, resource_activities as activities,
};
use application::{procedural_facts::*, resource_hearings::*};
use domain::{cases::CaseId, procedural_resources::ResourceId};
use serde_json::{json, Value};

pub(super) fn command(
    c: &ResourceHearingCommand,
    case: CaseId,
    resource: ResourceId,
) -> Result<Value, ApiError> {
    let act = c.act.map(|a| {
        json!({"id":a.id.to_string(),"revision":a.revision.get(),
        "resource_revision":a.resource_revision.get(),"capture_digest":a.capture_digest.to_hex()})
    });
    Ok(json!({"case_id":case,"resource_id":resource.to_string(),
        "operation_id":c.operation_id.to_string(),"hearing_id":c.hearing_id.to_string(),
        "association_id":c.association_id.to_string(),"expected_resource_revision":c.expected_resource_revision.get(),
        "resource":activities::project_resource(c.resource),"act":act,
        "values":activities::resource_hearing_values(&c.values)?}))
}
pub(super) fn project(
    row: ResourceHearingDraft,
    case: CaseId,
    resource: ResourceId,
    expected: &ResourceHearingCommand,
) -> Result<Value, ApiError> {
    if row.case_id != case
        || row.command != *expected
        || expected.resource.id != resource
        || row.observed_resource_head.id != resource
        || row.observed_resource_head.revision != expected.expected_resource_revision
        || expected.resource.revision > row.observed_resource_head.revision
        || expected
            .act
            .is_some_and(|a| a.resource_revision > row.observed_resource_head.revision)
        || expected.resource.revision == row.observed_resource_head.revision
            && expected.resource.capture_digest != row.observed_resource_head.capture_digest
        || expected.act.is_some_and(|a| {
            a.resource_revision == row.observed_resource_head.revision
                && a.capture_digest != row.observed_resource_head.capture_digest
        })
        || row.participants.len() != expected.values.participants().len()
    {
        return Err(ApiError::internal());
    }
    let support = expected.values.scheduling_basis().support();
    if row.support.reference != support.reference() || row.support.digest != support.digest() {
        return Err(ApiError::internal());
    }
    let mut admitted = false;
    for captured in row.resource.sources.supports.iter().chain(
        row.act
            .iter()
            .flat_map(|r| r.act.iter().flat_map(|a| a.supports.iter())),
    ) {
        if captured.reference == row.support.reference {
            if captured != &row.support {
                return Err(ApiError::internal());
            }
            admitted = true;
        }
    }
    if !admitted {
        return Err(ApiError::internal());
    }
    let mut participants = Vec::with_capacity(row.participants.len());
    for (person, selected) in row.participants.iter().zip(expected.values.participants()) {
        if person.snapshot.case_id != case
            || person.overview.case_id != case
            || person.snapshot.reference.id != selected.id()
            || person.overview.id != selected.id()
            || person.snapshot.reference.revision != selected.revision()
            || person.overview.revision != selected.revision()
        {
            return Err(ApiError::internal());
        }
        let mut component = empty();
        component.resolved.participants.push(person.snapshot);
        component.views.participants.push(person.overview.clone());
        participants.push(project_sources(&component)?["participants"][0].take());
    }
    let mut component = empty();
    component.direct_supports.push(row.support.clone());
    let support = project_sources(&component)?["direct_supports"][0].take();
    if row.resource.receipt.capture_digest != expected.resource.capture_digest {
        return Err(ApiError::internal());
    }
    let source = resources::exact_projection(
        row.resource,
        case,
        resource,
        Some(expected.resource.revision),
    )?;
    let act = match (expected.act, row.act) {
        (None, None) => None,
        (Some(reference), Some(value)) => {
            if value.receipt.capture_digest != reference.capture_digest
                || value
                    .act
                    .as_ref()
                    .is_none_or(|a| a.id != reference.id || a.revision != reference.revision)
            {
                return Err(ApiError::internal());
            }
            Some(resources::exact_projection(
                value,
                case,
                resource,
                Some(reference.resource_revision),
            )?)
        }
        _ => return Err(ApiError::internal()),
    };
    Ok(
        json!({"command":command(expected,case,resource)?,"resource":source,"act":act,
        "sources":{"support":support,"participants":participants},
        "recorded_by":resources::project_actor(&row.recorded_by)?,
        "observed_administration":resources::project_administration(&row.observed_administration,case)?,
        "observed_resource_head":activities::project_resource(row.observed_resource_head),
        "submission_digest":row.submission_digest.to_hex()}),
    )
}
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
