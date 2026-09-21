use super::{decode, inconsistent, port, stored_error};
use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

const BOUNDS: &str = "octet_length(r.selection_canonical) IN (111,167)
    AND octet_length(r.submission_canonical)>=5 AND octet_length(r.submission_canonical)<=1048576
    AND octet_length(r.capture_canonical)=49 AND octet_length(r.submission_digest)=32 AND octet_length(r.capture_digest)=32
    AND octet_length(r.resource_capture_digest)=32 AND octet_length(r.recorded_resource_capture_digest)=32
    AND coalesce(octet_length(r.act_capture_digest),0)<=32 AND coalesce(octet_length(r.hearing_submission_digest),0)<=32
    AND coalesce(octet_length(r.deadline_capture_digest),0)<=32 AND coalesce(octet_length(r.previous_capture_digest),0)<=32
    AND coalesce(octet_length(r.recorded_administration_digest),0)<=32 AND coalesce(octet_length(r.reason),0)<=4000
    AND coalesce(octet_length(r.recorded_administration_title),0)<=800 AND coalesce(octet_length(r.recorded_administration_reference),0)<=400
    AND octet_length(r.recorded_by_email)<=1280";

pub(crate) fn revision(value: i64) -> Result<ResourceActivityRevision, ApplicationError> {
    ResourceActivityRevision::new(u32::try_from(value).map_err(inconsistent)?).map_err(inconsistent)
}
pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    resource: ResourceId,
    id: ResourceActivityId,
    wanted: Option<ResourceActivityRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceActivityDetail, ApplicationError> {
    let current = raw(tx, case, resource, id, wanted, hasher)?;
    if let Some(reference) = current.receipt.previous {
        let previous =
            raw(tx, case, resource, id, Some(reference.revision), hasher).map_err(stored_error)?;
        if current.revision.get() != 2
            || previous.revision.get() != 1
            || previous.receipt.action != ResourceActivityAction::Link
            || previous.status != ResourceActivityStatus::Linked
            || current.receipt.action != ResourceActivityAction::Unlink
            || current.status != ResourceActivityStatus::Unlinked
            || reference.capture_digest != previous.receipt.capture_digest
            || current.recorded_at < previous.recorded_at
            || current.selection != previous.selection
            || current.sources != previous.sources
            || current.recorded_resource_head.revision < previous.recorded_resource_head.revision
            || (current.recorded_resource_head.revision == previous.recorded_resource_head.revision
                && current.recorded_resource_head != previous.recorded_resource_head)
        {
            return Err(inconsistent(
                "association successor changed its retained capture",
            ));
        }
        application::procedural_facts::validate_fact_administration(
            hasher,
            case,
            &current.recorded_administration,
            Some(&previous.recorded_administration),
        )
        .map_err(stored_error)?;
    }
    Ok(current)
}
fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    resource: ResourceId,
    id: ResourceActivityId,
    wanted: Option<ResourceActivityRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceActivityDetail, ApplicationError> {
    let wanted = wanted.map(|value| i64::from(value.get()));
    let probe = tx.query_opt(&format!("SELECT r.revision,({BOUNDS}) AS bounded,p.initial_revision
        FROM case_resource_activity_association_revisions r JOIN case_resource_activity_associations p
            ON p.id=r.association_id AND p.case_id=r.case_id AND p.resource_id=r.resource_id
        WHERE r.case_id=$1 AND r.resource_id=$2 AND r.association_id=$3 AND ($4::bigint IS NULL OR r.revision=$4)
        ORDER BY r.revision DESC LIMIT 1"), &[&case.as_uuid(),&resource.as_uuid(),&id.as_uuid(),&wanted]).map_err(port)?;
    let Some(probe) = probe else {
        if wanted.is_none() && tx.query_opt("SELECT id FROM case_resource_activity_associations WHERE id=$1 AND case_id=$2 AND resource_id=$3",
            &[&id.as_uuid(),&case.as_uuid(),&resource.as_uuid()]).map_err(port)?.is_some() {
            return Err(inconsistent("association root has no initial revision"));
        }
        return Err(ResourceActivityError::NotFound.into());
    };
    if !probe.try_get::<_, bool>("bounded").map_err(inconsistent)?
        || probe.get::<_, i64>("initial_revision") != 1
    {
        return Err(inconsistent("association capture or root exceeds bounds"));
    }
    let revision: i64 = probe.get("revision");
    let row = tx.query_opt(&format!("SELECT r.* FROM case_resource_activity_association_revisions r
        WHERE r.case_id=$1 AND r.resource_id=$2 AND r.association_id=$3 AND r.revision=$4 AND ({BOUNDS})"),
        &[&case.as_uuid(),&resource.as_uuid(),&id.as_uuid(),&revision]).map_err(port)?.ok_or_else(||inconsistent("association capture changed during read"))?;
    decode::detail(tx, &row, hasher)
}
