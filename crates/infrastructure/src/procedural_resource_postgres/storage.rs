use super::{decode, inconsistent, port};
use application::{procedural_resources::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

const BOUNDS:&str="octet_length(r.values_canonical) BETWEEN 5 AND 262144 AND octet_length(r.values_view::text)<=524288
 AND octet_length(r.sources_canonical) BETWEEN 5 AND 524288 AND octet_length(r.supports_view::text)<=16384
 AND octet_length(r.submission_canonical) BETWEEN 5 AND 1048576 AND octet_length(r.capture_canonical)=57
 AND octet_length(r.values_digest)=32 AND octet_length(r.sources_digest)=32 AND octet_length(r.submission_digest)=32 AND octet_length(r.capture_digest)=32
 AND coalesce(octet_length(r.act_values_canonical),0)<=32768 AND coalesce(octet_length(r.act_values_view::text),0)<=65536
 AND coalesce(octet_length(r.act_supports_view::text),0)<=16384 AND octet_length(r.recorded_by_email)<=1280
 AND coalesce(octet_length(r.reason),0)<=4000 AND coalesce(octet_length(r.recorded_administration_title),0)<=800
 AND coalesce(octet_length(r.recorded_administration_reference),0)<=400";

pub(crate) fn revision(value: i64) -> Result<ResourceRevision, ApplicationError> {
    ResourceRevision::new(u32::try_from(value).map_err(inconsistent)?).map_err(inconsistent)
}
pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ResourceId,
    wanted: Option<ResourceRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceDetail, ApplicationError> {
    let current = raw(tx, case, id, wanted, hasher)?;
    if let Some(reference) = current.receipt.previous {
        let previous = raw(tx, case, id, Some(reference.revision), hasher)?;
        if previous.receipt.capture_digest != reference.capture_digest
            || previous.recorded_at > current.recorded_at
        {
            return Err(inconsistent(
                "resource predecessor receipt or clock differs",
            ));
        }
        match (previous.status, current.receipt.action) {
            (ResourceStatus::Archived, ResourceAction::Reactivate) => {}
            (ResourceStatus::Archived, _)
            | (ResourceStatus::Active, ResourceAction::Reactivate) => {
                return Err(inconsistent("resource state transition differs"))
            }
            _ => {}
        }
        if !matches!(current.receipt.action, ResourceAction::Correct)
            && (current.values != previous.values || current.sources != previous.sources)
        {
            return Err(inconsistent("resource retained capture changed"));
        }
        retained(&previous.sources.supports, &current.sources.supports)?;
        if current.sources.resolution.snapshot.reference
            == previous.sources.resolution.snapshot.reference
            && current.sources.resolution != previous.sources.resolution
        {
            return Err(inconsistent("resource retained resolution differs"));
        }
        for participant in &current.sources.appellants {
            if previous
                .sources
                .appellants
                .iter()
                .any(|p| p.snapshot.reference == participant.snapshot.reference && p != participant)
            {
                return Err(inconsistent("retained resource participant differs"));
            }
        }
        application::procedural_facts::validate_fact_administration(
            hasher,
            case,
            &current.recorded_administration,
            Some(&previous.recorded_administration),
        )?;
        match (
            previous.recorded_stage.entry(),
            current.recorded_stage.entry(),
        ) {
            (Some(_), None) => return Err(inconsistent("resource stage observation regressed")),
            (Some(old), Some(new))
                if new.stage_revision() < old.stage_revision()
                    || (new.stage_revision() == old.stage_revision() && new != old) =>
            {
                return Err(inconsistent("resource stage observation differs"))
            }
            _ => {}
        }
    }
    if let Some(act) = &current.act {
        let root=tx.query_opt("SELECT a.initial_resource_revision FROM case_procedural_resource_acts a
            JOIN case_procedural_resource_revisions r ON r.resource_id=a.resource_id AND r.case_id=a.case_id
                AND r.revision=a.initial_resource_revision AND r.act_id=a.id AND r.act_revision=1 AND r.action='record_act'
            WHERE a.id=$1 AND a.resource_id=$2 AND a.case_id=$3",
            &[&act.id.as_uuid(),&id.as_uuid(),&case.as_uuid()]).map_err(port)?.ok_or_else(||inconsistent("resource act root absent"))?;
        if act.revision.get() == 1 && root.get::<_, i64>(0) != i64::from(current.revision.get()) {
            return Err(inconsistent("resource act initial revision differs"));
        }
        if let Some(reference) = act.previous {
            let previous = raw(tx, case, id, Some(reference.revision), hasher)?;
            let old = previous
                .act
                .as_ref()
                .ok_or_else(|| inconsistent("resource act predecessor is absent"))?;
            if previous.receipt.capture_digest != reference.capture_digest
                || old.id != act.id
                || old.revision.next() != Some(act.revision)
            {
                return Err(inconsistent("resource act predecessor differs"));
            }
            retained(&old.supports, &act.supports)?;
        }
    }
    Ok(current)
}
fn retained(
    old: &[application::case_stages::StageSupportSnapshot],
    new: &[application::case_stages::StageSupportSnapshot],
) -> Result<(), ApplicationError> {
    if new
        .iter()
        .any(|n| old.iter().any(|o| o.reference == n.reference && o != n))
    {
        return Err(inconsistent("retained resource support differs"));
    }
    Ok(())
}
fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ResourceId,
    wanted: Option<ResourceRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceDetail, ApplicationError> {
    let wanted = wanted.map(|r| i64::from(r.get()));
    let probe=tx.query_opt(&format!("SELECT r.revision,({BOUNDS}) AS bounded FROM case_procedural_resource_revisions r JOIN case_procedural_resources p ON p.id=r.resource_id AND p.case_id=r.case_id WHERE r.case_id=$1 AND r.resource_id=$2 AND ($3::bigint IS NULL OR r.revision=$3) ORDER BY r.revision DESC LIMIT 1"),
        &[&case.as_uuid(),&id.as_uuid(),&wanted]).map_err(port)?.ok_or(ProceduralResourceError::NotFound)?;
    if !probe.try_get::<_, bool>(1).map_err(inconsistent)? {
        return Err(inconsistent("resource captured record exceeds bounds"));
    }
    let revision: i64 = probe.get(0);
    let row=tx.query_opt(&format!("SELECT r.* FROM case_procedural_resource_revisions r WHERE case_id=$1 AND resource_id=$2 AND revision=$3 AND {BOUNDS}"),
        &[&case.as_uuid(),&id.as_uuid(),&revision]).map_err(port)?.ok_or_else(||inconsistent("resource capture changed during read"))?;
    decode::detail(tx, &row, hasher)
}
