use super::{administration, decode, inconsistent, port, sources, target};
use application::{procedural_facts::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher};
use postgres::Transaction;

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    target: FactTarget,
    revision: Option<FactRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<FactDetail, ApplicationError> {
    let current = raw(tx, case, target, revision, hasher)?;
    sources::validate(tx, &current, hasher)?;
    let m = current.snapshot.metadata();
    if m.revision.get() > 1 {
        let previous = raw(
            tx,
            case,
            target,
            Some(FactRevision::new(m.revision.get() - 1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(|e| match e {
            ApplicationError::ProceduralFact(ProceduralFactError::NotFound) => {
                inconsistent("fact predecessor missing")
            }
            other => other,
        })?;
        if previous.snapshot.metadata().status != FactStatus::Recorded {
            return Err(inconsistent("fact predecessor was withdrawn"));
        }
        validate_fact_administration(
            hasher,
            case,
            &m.recorded_administration,
            Some(&previous.snapshot.metadata().recorded_administration),
        )?;
        validate_fact_retained_sources(&previous.sources, &current.sources)?;
        if m.receipt.action == FactAction::Withdraw
            && (previous.snapshot.values() != current.snapshot.values()
                || previous.sources != current.sources)
        {
            return Err(inconsistent("fact withdrawal changed values or sources"));
        }
    }
    Ok(current)
}
fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    wanted: FactTarget,
    revision: Option<FactRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<FactDetail, ApplicationError> {
    let (family, id, parent) = target::parts(wanted);
    let revision = revision.map(|r| i64::from(r.get()));
    let probe=tx.query_opt("SELECT r.revision,
        octet_length(r.values_canonical) BETWEEN 27 AND 58671 AND octet_length(r.sources_canonical) BETWEEN 19 AND 36847
        AND octet_length(r.values_digest)=32 AND octet_length(r.sources_digest)=32 AND octet_length(r.submission_digest)=32
        AND (r.recorded_administration_digest IS NULL OR octet_length(r.recorded_administration_digest)=32)
        AND octet_length(r.submission_canonical) BETWEEN 141 AND 4161 AND octet_length(r.values_view::text)<=1048576
        AND octet_length(r.sources_view::text)<=1048576 AND octet_length(r.submission_view::text)<=32768
        AND octet_length(r.recorded_by_email)<=1280 AND COALESCE(octet_length(r.reason),0)<=4000
        AND COALESCE(octet_length(r.recorded_administration_title),0)<=800
        AND COALESCE(octet_length(r.recorded_administration_reference),0)<=400 AS bounded
        FROM case_procedural_fact_revisions r JOIN case_procedural_facts f USING(family,id,case_id)
        WHERE r.family=$1 AND r.id=$2 AND r.case_id=$3 AND f.parent_resolution_id IS NOT DISTINCT FROM $4::uuid
        AND ($5::bigint IS NULL OR r.revision=$5) ORDER BY r.revision DESC LIMIT 1", &[&family,&id,&case.as_uuid(),&parent,&revision]).map_err(port)?.ok_or(ProceduralFactError::NotFound)?;
    if !probe.try_get::<_, bool>("bounded").map_err(inconsistent)? {
        return Err(inconsistent("persisted fact fields exceed bounds"));
    }
    let selected: i64 = probe.try_get("revision").map_err(inconsistent)?;
    let row=tx.query_opt("SELECT r.*,f.parent_resolution_id,f.parent_family,f.initial_revision FROM case_procedural_fact_revisions r
        JOIN case_procedural_facts f USING(family,id,case_id) WHERE r.family=$1 AND r.id=$2 AND r.case_id=$3
        AND f.parent_resolution_id IS NOT DISTINCT FROM $4::uuid AND r.revision=$5
        AND octet_length(r.values_canonical) BETWEEN 27 AND 58671 AND octet_length(r.sources_canonical) BETWEEN 19 AND 36847
        AND octet_length(r.values_digest)=32 AND octet_length(r.sources_digest)=32 AND octet_length(r.submission_digest)=32
        AND (r.recorded_administration_digest IS NULL OR octet_length(r.recorded_administration_digest)=32)
        AND octet_length(r.submission_canonical) BETWEEN 141 AND 4161 AND octet_length(r.values_view::text)<=1048576
        AND octet_length(r.sources_view::text)<=1048576 AND octet_length(r.submission_view::text)<=32768
        AND octet_length(r.recorded_by_email)<=1280 AND COALESCE(octet_length(r.reason),0)<=4000
        AND COALESCE(octet_length(r.recorded_administration_title),0)<=800
        AND COALESCE(octet_length(r.recorded_administration_reference),0)<=400", &[&family,&id,&case.as_uuid(),&parent,&selected]).map_err(port)?.ok_or_else(||inconsistent("immutable fact changed during read"))?;
    let captured = administration::captured(tx, &row, case, hasher)?;
    let detail = decode::row(&row, captured, hasher)?;
    if detail.snapshot.target() != wanted {
        return Err(inconsistent("fact root differs from requested target"));
    }
    Ok(detail)
}
