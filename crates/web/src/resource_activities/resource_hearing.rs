use super::{projection, selection, sources};
use crate::{
    error::ApiError, procedural_facts::sources::project, procedural_resources as resources,
};
use application::{procedural_facts::*, resource_activities::*, resource_hearings::*};
use domain::{
    cases::CaseId,
    procedural_facts::FactDeclaration,
    procedural_resources::ResourceStatus,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision, ResourceHearingValues},
};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

/// Project the verified capture as its own family, with no ordinary stage context.
pub(super) fn detail(
    row: ResourceHearingDetail,
    case: CaseId,
    resource: ResourceId,
    id: ResourceHearingId,
    revision: Option<ResourceHearingRevision>,
) -> Result<Value, ApiError> {
    validate(&row, case, resource, id, revision)?;
    let review = &row.review;
    let command = &review.command;
    let selected = ResourceActivitySelection {
        resource: command.resource,
        act: command.act,
        target: ResourceActivityTarget::ResourceHearing {
            id,
            revision: row.revision,
            capture_digest: row.capture_digest,
        },
    };
    let (source, act) = sources::resource_sources(
        review.resource.clone(),
        review.act.clone(),
        case,
        resource,
        selected,
    )?;
    let mut component = empty();
    component.direct_supports.push(review.support.clone());
    let support = project(&component)?["direct_supports"][0].take();
    let mut participants = Vec::with_capacity(review.participants.len());
    for person in &review.participants {
        let mut component = empty();
        component.resolved.participants.push(person.snapshot);
        component.views.participants.push(person.overview.clone());
        participants.push(project(&component)?["participants"][0].take());
    }
    let references = selection::project(selected);
    Ok(json!({
        "case_id":case,"resource_id":resource.to_string(),"id":id.to_string(),
        "revision":row.revision.get(),"operation_id":command.operation_id.to_string(),
        "association_id":command.association_id.to_string(),
        "expected_resource_revision":command.expected_resource_revision.get(),
        "resource":references["resource"],"act":references["act"],
        "values":values(&command.values)?,
        "sources":{"resource":source,"act":act,"support":support,"participants":participants},
        "recorded_by":resources::project_actor(&review.recorded_by)?,
        "recorded_at":resources::utc(row.recorded_at)?,
        "recorded_administration":resources::project_administration(&review.observed_administration,case)?,
        "recorded_resource_head":selection::resource(review.observed_resource_head),
        "submission_digest":review.submission_digest.to_hex(),"capture_digest":row.capture_digest.to_hex()
    }))
}

fn validate(
    row: &ResourceHearingDetail,
    case: CaseId,
    resource: ResourceId,
    id: ResourceHearingId,
    revision: Option<ResourceHearingRevision>,
) -> Result<(), ApiError> {
    let d = &row.review;
    let c = &d.command;
    let m = &row.material;
    let h = &m.resource_head;
    if d.case_id != case
        || m.case_id != case
        || h.case_id != case
        || c.resource.id != resource
        || h.id != resource
        || c.hearing_id != id
        || row.revision != ResourceHearingRevision::initial()
        || revision.is_some_and(|v| v != row.revision)
        || d.resource != m.resource
        || d.act != m.act
        || d.observed_administration != m.administration
        || d.observed_resource_head
            != (ResourceCaptureRef {
                id: h.id,
                revision: h.revision,
                capture_digest: h.receipt.capture_digest,
            })
        || c.expected_resource_revision != h.revision
        || c.resource.revision > h.revision
        || c.act.is_some_and(|a| a.resource_revision > h.revision)
        || h.status != ResourceStatus::Active
        || row.recorded_at < h.recorded_at
        || row.recorded_at < d.resource.recorded_at
        || d.act
            .as_ref()
            .is_some_and(|a| row.recorded_at < a.recorded_at)
        || d.observed_administration
            .snapshot()
            .is_some_and(|a| row.recorded_at < a.changed_at)
    {
        return Err(ApiError::internal());
    }
    projection::instant(row.recorded_at)?;
    for source in [&d.resource, h] {
        let FactDeclaration::Known(mode) = source.values.mode() else {
            return Err(ApiError::internal());
        };
        if !c
            .values
            .kind()
            .is_compatible_with(source.values.kind(), *mode)
        {
            return Err(ApiError::internal());
        }
    }
    let support = c.values.scheduling_basis().support();
    if d.support.reference != support.reference() || d.support.digest != support.digest() {
        return Err(ApiError::internal());
    }
    let mut admitted = false;
    for captured in d.resource.sources.supports.iter().chain(
        d.act
            .iter()
            .flat_map(|r| r.act.iter().flat_map(|a| a.supports.iter())),
    ) {
        if captured.reference == d.support.reference {
            if captured != &d.support {
                return Err(ApiError::internal());
            }
            admitted = true;
        }
    }
    if !admitted
        || d.participants.len() != c.values.participants().len()
        || m.participants.len() != d.participants.len()
    {
        return Err(ApiError::internal());
    }
    let mut material = m.participants.iter().collect::<Vec<_>>();
    material.sort_by_key(|p| p.id().as_uuid());
    for ((p, r), source) in d
        .participants
        .iter()
        .zip(c.values.participants())
        .zip(material)
    {
        if p.snapshot.case_id != case
            || p.snapshot.reference.id != r.id()
            || p.snapshot.reference.revision != r.revision()
            || p.overview.case_id != case
            || p.overview.id != r.id()
            || p.overview.revision != r.revision()
            || source.case_id() != case
            || source.id() != r.id()
            || source.revision_number() != r.revision()
        {
            return Err(ApiError::internal());
        }
    }
    Ok(())
}

fn values(v: &ResourceHearingValues) -> Result<Value, ApiError> {
    let basis = v.scheduling_basis();
    let support = basis.support();
    let scheduled = v
        .scheduled_at()
        .value()
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())?;
    Ok(json!({"kind":v.kind().as_str(),"scheduled_at":scheduled,
        "modality":v.modality().as_str(),"venue":v.venue().as_str(),
        "note":v.note().map(|n|n.as_str()),
        "participants":v.participants().iter().map(|p|json!({
            "participant_id":p.id().to_string(),"revision":p.revision().get()})).collect::<Vec<_>>(),
        "scheduling_basis":{"statement":basis.statement().as_str(),"support":{
            "document_id":support.reference().id.to_string(),"version":support.reference().version.get(),
            "digest":support.digest().to_hex()}}
    }))
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
