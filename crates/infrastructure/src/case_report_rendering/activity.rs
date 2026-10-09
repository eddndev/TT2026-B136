use super::*;
use domain::{cases::CaseId, identity::UserId};

pub(super) struct Totals {
    pub capture: [u64; 3],
    pub actors: Vec<[u64; 3]>,
}

pub(super) fn payload(
    snapshot: &CaseReportSnapshot,
) -> Result<&CaseReportActivitySnapshot, ApplicationError> {
    snapshot.activity.as_ref().ok_or_else(failed)
}

pub(super) fn totals(value: &CaseReportActivitySnapshot) -> Result<Totals, ApplicationError> {
    let mut result = Totals {
        capture: [0; 3],
        actors: vec![[0; 3]; value.actors.len()],
    };
    for row in &value.rows {
        let index = value
            .actors
            .binary_search_by_key(&row.litigator_id.as_uuid(), |who| who.user_id.as_uuid())
            .map_err(|_| failed())?;
        for (metric, amount) in counts(row).into_iter().enumerate() {
            result.capture[metric] = result.capture[metric]
                .checked_add(amount)
                .ok_or_else(capacity)?;
            result.actors[index][metric] = result.actors[index][metric]
                .checked_add(amount)
                .ok_or_else(capacity)?;
        }
    }
    Ok(result)
}

pub(super) fn counts(row: &CaseReportActivityRow) -> [u64; 3] {
    [
        row.documents_uploaded,
        row.procedural_activities,
        row.deadlines_attended,
    ]
}

pub(super) fn text(counts: [u64; 3]) -> String {
    format!(
        "Documentos: {}    Actuaciones: {}    Plazos atendidos: {}",
        counts[0], counts[1], counts[2]
    )
}

pub(super) fn case(
    snapshot: &CaseReportSnapshot,
    id: CaseId,
) -> Result<&CaseReportRow, ApplicationError> {
    let index = snapshot
        .cases
        .binary_search_by_key(&id.as_uuid(), |case| case.case_id.as_uuid())
        .map_err(|_| failed())?;
    Ok(&snapshot.cases[index])
}

pub(super) fn actor(
    value: &CaseReportActivitySnapshot,
    id: UserId,
) -> Result<&CaseReportLitigator, ApplicationError> {
    let index = value
        .actors
        .binary_search_by_key(&id.as_uuid(), |who| who.user_id.as_uuid())
        .map_err(|_| failed())?;
    Ok(&value.actors[index])
}
