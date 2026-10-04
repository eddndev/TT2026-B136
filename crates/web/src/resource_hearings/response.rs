use crate::{error::ApiError, resource_activities as activities};
use application::{resource_activities::*, resource_hearings::*};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};
use serde_json::{json, Value};

pub(super) fn submitted(
    row: ResourceHearingCreation,
    case: CaseId,
    resource: ResourceId,
    expected: &ResourceHearingCommand,
    digest: Sha256Digest,
) -> Result<Value, ApiError> {
    if row.hearing.review.command != *expected || row.hearing.review.submission_digest != digest {
        return Err(ApiError::internal());
    }
    creation(
        row,
        case,
        resource,
        expected.hearing_id,
        Some(ResourceHearingRevision::initial()),
    )
}
pub(super) fn creation(
    mut row: ResourceHearingCreation,
    case: CaseId,
    resource: ResourceId,
    id: ResourceHearingId,
    revision: Option<ResourceHearingRevision>,
) -> Result<Value, ApiError> {
    row.hearing
        .material
        .participants
        .sort_by_key(|p| (p.id().as_uuid(), p.revision_number().get()));
    if let ResourceActivityTargetDetail::ResourceHearing(target) =
        &mut row.association.sources.target
    {
        target
            .material
            .participants
            .sort_by_key(|p| (p.id().as_uuid(), p.revision_number().get()));
    }
    let h = &row.hearing;
    let d = &h.review;
    let c = &d.command;
    let origin = ResourceHearingOrigin {
        case_id: case,
        resource_id: resource,
        hearing_id: id,
        operation_id: c.operation_id,
        association_id: c.association_id,
        submission_digest: d.submission_digest,
        capture_digest: h.capture_digest,
    };
    let link = ResourceActivityCommand {
        operation_id: ResourceActivityOperationId::from_uuid(c.operation_id.as_uuid()),
        association_id: c.association_id,
        expected_resource_revision: c.expected_resource_revision,
        change: ResourceActivityChange::Link {
            selection: ResourceActivitySelection {
                resource: c.resource,
                act: c.act,
                target: ResourceActivityTarget::ResourceHearing {
                    id,
                    revision: h.revision,
                    capture_digest: h.capture_digest,
                },
            },
        },
    };
    let a = &row.association;
    if row.origin != origin
        || d.case_id != case
        || c.resource.id != resource
        || c.hearing_id != id
        || revision.is_some_and(|v| v != h.revision)
        || a.revision != ResourceActivityRevision::initial()
        || resource_activity_command_from_detail(a).map_err(|_| ApiError::internal())? != link
        || a.sources.target != ResourceActivityTargetDetail::ResourceHearing(Box::new(h.clone()))
        || a.sources.resource != d.resource
        || a.sources.act != d.act
        || a.recorded_at != h.recorded_at
        || a.recorded_by != d.recorded_by
        || a.recorded_administration != d.observed_administration
        || a.recorded_resource_head != d.observed_resource_head
    {
        return Err(ApiError::internal());
    }
    let hearing =
        activities::resource_hearing_projection(row.hearing, case, resource, id, revision)?;
    let association = activities::exact_projection(
        row.association,
        case,
        resource,
        origin.association_id,
        Some(ResourceActivityRevision::initial()),
    )?;
    Ok(json!({"hearing":hearing,"association":association,
        "origin":{"case_id":origin.case_id,"resource_id":origin.resource_id.to_string(),
            "hearing_id":origin.hearing_id.to_string(),"operation_id":origin.operation_id.to_string(),
            "association_id":origin.association_id.to_string(),"submission_digest":origin.submission_digest.to_hex(),
            "capture_digest":origin.capture_digest.to_hex()},
        "submission_digest":origin.submission_digest.to_hex()}))
}
pub(super) fn page(
    page: ResourceHearingPage,
    case: CaseId,
    resource: ResourceId,
    limit: u16,
    after: Option<ResourceHearingId>,
) -> Result<Value, ApiError> {
    let id = |row: &ResourceHearingCreation| row.hearing.review.command.hearing_id;
    if page.case_id != case
        || page.resource_id != resource
        || page.items.len() > usize::from(limit)
        || page.has_more != page.next_after_id.is_some()
        || page.has_more
            && (page.items.len() != usize::from(limit)
                || page.items.last().map(id) != page.next_after_id)
        || page
            .items
            .iter()
            .any(|row| after.is_some_and(|cursor| id(row).as_uuid() <= cursor.as_uuid()))
        || page
            .items
            .windows(2)
            .any(|p| id(&p[0]).as_uuid() >= id(&p[1]).as_uuid())
    {
        return Err(ApiError::internal());
    }
    let items = page
        .items
        .into_iter()
        .map(|row| {
            let hearing = id(&row);
            creation(row, case, resource, hearing, None)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"case_id":case,"resource_id":resource.to_string(),"items":items,
        "has_more":page.has_more,"next_after_id":page.next_after_id.map(|v|v.to_string())}),
    )
}
