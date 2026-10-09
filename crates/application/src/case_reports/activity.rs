use super::{canonical::Encoder, checks::*, *};
use crate::ApplicationError;

pub(super) fn validate(snapshot: &CaseReportSnapshot) -> Result<(), ApplicationError> {
    let value = snapshot
        .activity
        .as_ref()
        .ok_or_else(|| inconsistent("activity snapshot has no captured activity"))?;
    if value.actors.len() > MAX_REPORT_WORKLOAD || value.rows.len() > MAX_REPORT_ASSIGNMENTS {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    let mut prior_actor = None;
    for who in &value.actors {
        let id = who.user_id.as_uuid();
        email(&who.email)?;
        if id.is_nil() || prior_actor.is_some_and(|prior| prior >= id) {
            return Err(inconsistent("activity actors are not strictly ordered"));
        }
        prior_actor = Some(id);
    }
    let mut prior_row = None;
    let mut totals = [0_u64; 3];
    for row in &value.rows {
        let key = (row.case_id.as_uuid(), row.litigator_id.as_uuid());
        if prior_row.is_some_and(|prior| prior >= key)
            || snapshot
                .cases
                .binary_search_by_key(&key.0, |case| case.case_id.as_uuid())
                .is_err()
            || value
                .actors
                .binary_search_by_key(&key.1, |who| who.user_id.as_uuid())
                .is_err()
            || snapshot
                .filters
                .litigator
                .is_some_and(|id| id != row.litigator_id)
        {
            return Err(inconsistent(
                "activity row order, captured references or author filter differs",
            ));
        }
        prior_row = Some(key);
        for (total, amount) in totals.iter_mut().zip([
            row.documents_uploaded,
            row.procedural_activities,
            row.deadlines_attended,
        ]) {
            *total = total
                .checked_add(amount)
                .ok_or(CaseReportError::CapacityExceeded)?;
        }
    }
    Ok(())
}

pub(super) fn encode(
    bytes: &mut Encoder,
    value: &CaseReportActivitySnapshot,
) -> Result<(), ApplicationError> {
    bytes.number(u64::from(value.documents_complete))?;
    bytes.number(value.actors.len() as u64)?;
    for who in &value.actors {
        bytes.litigator(who)?;
    }
    bytes.number(value.rows.len() as u64)?;
    for row in &value.rows {
        bytes.uuid(row.case_id.as_uuid())?;
        bytes.uuid(row.litigator_id.as_uuid())?;
        bytes.number(row.documents_uploaded)?;
        bytes.number(row.procedural_activities)?;
        bytes.number(row.deadlines_attended)?;
    }
    Ok(())
}
