use super::{port, target};
use application::{cases::CurrentCaseAdministration, procedural_facts::*, ApplicationError};
use postgres::Transaction;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    detail: &FactDetail,
    command: &ProceduralFactCommand,
) -> Result<(), ApplicationError> {
    let s = &detail.snapshot;
    let m = s.metadata();
    let (family, id, parent) = target::parts(s.target());
    if command.action() == FactAction::Record {
        tx.execute("INSERT INTO case_procedural_facts(family,id,case_id,parent_resolution_id) VALUES($1,$2,$3,$4)",&[&family,&id,&s.case_id().as_uuid(),&parent]).map_err(port)?;
    }
    let canonical = target::canonical(&s.values());
    let sources = fact_sources_bytes(&detail.sources)?;
    let submission = fact_submission_bytes(
        m.recorded_by.id,
        s.case_id(),
        command,
        m.values_digest,
        m.receipt.sources_digest,
    )?;
    let (revision, digest, title, reference) = match &m.recorded_administration {
        CurrentCaseAdministration::Unrevised(v) => {
            (None, None, Some(v.title()), Some(v.reference()))
        }
        CurrentCaseAdministration::Recorded(v) => (
            Some(i64::from(v.revision.get())),
            Some(v.values_digest.as_bytes().as_slice()),
            None,
            None,
        ),
    };
    tx.execute("INSERT INTO case_procedural_fact_revisions(family,id,case_id,revision,values_canonical,values_digest,
        sources_canonical,sources_digest,operation_id,action,reason,submission_canonical,submission_digest,
        recorded_administration_revision,recorded_administration_digest,recorded_administration_title,recorded_administration_reference,
        recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)",
        &[&family,&id,&s.case_id().as_uuid(),&i64::from(m.revision.get()),&canonical,&m.values_digest.as_bytes().as_slice(),
        &sources,&m.receipt.sources_digest.as_bytes().as_slice(),&m.receipt.operation_id.as_uuid(),&target::action(m.receipt.action),&m.reason.as_ref().map(FactText::as_str),
        &submission,&m.receipt.submission_digest.as_bytes().as_slice(),&revision,&digest,&title,&reference,&m.recorded_at.unix_timestamp(),&(m.recorded_at.nanosecond() as i32),&m.recorded_by.id.as_uuid(),&m.recorded_by.email]).map_err(port)?;
    Ok(())
}
pub(super) fn resource(s: &ProceduralFactSnapshot) -> String {
    let (family, id, _) = target::parts(s.target());
    let m = s.metadata();
    format!(
        "case:{}:{family}:{id}:revision:{}:operation:{}:sha256:{}",
        s.case_id(),
        m.revision.get(),
        m.receipt.operation_id,
        m.receipt.submission_digest.to_hex()
    )
}
