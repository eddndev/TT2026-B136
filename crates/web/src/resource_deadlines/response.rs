use crate::{
    deadlines, error::ApiError, procedural_resources as resources,
    resource_activities as activities,
};
use application::{deadlines::*, resource_activities::*, resource_deadlines::*};
use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};

fn association_command(
    command: &ResourceDeadlineCommand,
    id: DeadlineId,
    capture_digest: Sha256Digest,
) -> ResourceActivityCommand {
    let deadline = command.deadline.clone().into_parts().0;
    ResourceActivityCommand {
        association_id: command.association_id,
        operation_id: ResourceActivityOperationId::from_uuid(deadline.operation_id.as_uuid()),
        expected_resource_revision: command.expected_resource_revision,
        change: ResourceActivityChange::Link {
            selection: ResourceActivitySelection {
                resource: command.resource,
                act: command.act,
                target: ResourceActivityTarget::Deadline {
                    id,
                    revision: DeadlineRevision::initial(),
                    capture_digest,
                },
            },
        },
    }
}
fn command_value(
    command: &ResourceDeadlineCommand,
    case: CaseId,
    resource: ResourceId,
    deadline: Value,
) -> Value {
    let act=command.act.map(|a|json!({"id":a.id.to_string(),"revision":a.revision.get(),"resource_revision":a.resource_revision.get(),"capture_digest":a.capture_digest.to_hex()}));
    json!({"case_id":case,"resource_id":resource.to_string(),"association_id":command.association_id.to_string(),
        "expected_resource_revision":command.expected_resource_revision.get(),"resource":activities::project_resource(command.resource),"act":act,"deadline":deadline})
}
pub(super) fn draft(
    row: ResourceDeadlineDraft,
    case: CaseId,
    resource: ResourceId,
    expected: &ResourceDeadlineCommand,
) -> Result<Value, ApiError> {
    let a = row.association;
    let d = row.deadline;
    let link = association_command(expected, d.command.deadline_id, d.capture_digest);
    if row.command != *expected
        || a.command != link
        || a.recorded_by.id != d.actor
        || d.author
            != (DeadlineActorSnapshot::User {
                id: a.recorded_by.id,
                email: a.recorded_by.email.clone(),
            })
        || a.observed_administration != d.calculation.material.administration
        || a.observed_resource_head.revision != expected.expected_resource_revision
    {
        return Err(ApiError::internal());
    }
    let ResourceActivityChange::Link { selection } = link.change else {
        return Err(ApiError::internal());
    };
    activities::validate_context(
        selection,
        a.observed_resource_head,
        resource,
        ResourceActivityRevision::initial(),
        None,
    )?;
    let (source, act) = activities::resource_sources(a.resource, a.act, case, resource, selection)?;
    let deadline = deadlines::draft_projection(d, case, &expected.deadline)?;
    Ok(
        json!({"command":command_value(expected,case,resource,deadline["command"].clone()),
        "deadline":deadline,
        "association":{"command":activities::project_command(&a.command,case,resource),"resource":source,"act":act,
            "recorded_by":resources::project_actor(&a.recorded_by)?,
            "observed_administration":resources::project_administration(&a.observed_administration,case)?,
            "observed_resource_head":activities::project_resource(a.observed_resource_head),"submission_digest":a.submission_digest.to_hex()},
        "submission_digest":row.submission_digest.to_hex()}),
    )
}
pub(super) fn submitted(
    row: ResourceDeadlineResult,
    case: CaseId,
    resource: ResourceId,
    expected: &ResourceDeadlineCommand,
    digest: Sha256Digest,
) -> Result<Value, ApiError> {
    let d = row.deadline;
    let a = row.association;
    let link = association_command(expected, d.id, d.receipt.capture_digest);
    if row.submission_digest != digest
        || resource_activity_command_from_detail(&a).map_err(|_| ApiError::internal())? != link
        || a.sources.target != ResourceActivityTargetDetail::Deadline(Box::new(d.clone()))
        || a.recorded_administration != d.calculation.material.administration
        || a.recorded_at != d.recorded_at
        || d.recorded_by
            != (DeadlineActorSnapshot::User {
                id: a.recorded_by.id,
                email: a.recorded_by.email.clone(),
            })
    {
        return Err(ApiError::internal());
    }
    deadlines::validate_submission(&d, &expected.deadline, d.receipt.submission_digest)?;
    let id = expected.deadline.clone().into_parts().0.deadline_id;
    Ok(
        json!({"deadline":deadlines::exact_projection(d,case,id,Some(DeadlineRevision::initial()))?,
        "association":activities::exact_projection(a,case,resource,expected.association_id,Some(ResourceActivityRevision::initial()))?,
        "submission_digest":digest.to_hex()}),
    )
}
