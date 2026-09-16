use super::port;
use application::ApplicationError;
use domain::{
    cases::CaseId,
    hearing_results::{HearingResultId, HearingResultRevision},
    hearings::HearingId,
};
use postgres::{Client, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let broken: bool = tx
        .query_one(
            "SELECT
        EXISTS(SELECT 1 FROM case_hearing_results h LEFT JOIN case_hearing_result_revisions r
            ON r.result_id=h.id AND r.revision=1 WHERE r.result_id IS NULL
                OR h.initial_revision<>1 OR r.case_id<>h.case_id OR r.hearing_id<>h.hearing_id)
        OR EXISTS(SELECT 1 FROM case_hearing_result_revisions r LEFT JOIN case_hearing_results h
            ON h.id=r.result_id AND h.case_id=r.case_id AND h.hearing_id=r.hearing_id
            WHERE h.id IS NULL OR octet_length(r.values_canonical) NOT BETWEEN 26 AND 146933
                OR octet_length(r.submission_canonical) NOT BETWEEN 192 AND 4296
                OR r.values_digest<>sha256(r.values_canonical)
                OR r.submission_digest<>sha256(r.submission_canonical))
        OR EXISTS(SELECT 1 FROM case_hearing_result_revisions GROUP BY result_id
            HAVING min(revision)<>1 OR max(revision)<>count(*))",
            &[],
        )
        .map_err(port)?
        .get(0);
    if broken {
        return Err(inconsistent());
    }
    validate_continuations(&mut tx)?;
    let mut id: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows = tx.query(
            "SELECT result_id,revision,case_id,hearing_id
             FROM case_hearing_result_revisions WHERE $1::uuid IS NULL OR (result_id,revision)>($1,$2)
             ORDER BY result_id,revision LIMIT 64",
            &[&id, &revision],
        ).map_err(port)?;
        for row in &rows {
            let case = CaseId::from_uuid(row.try_get(2).map_err(|_| inconsistent())?);
            let hearing = HearingId::from_uuid(row.try_get(3).map_err(|_| inconsistent())?);
            let result = HearingResultId::from_uuid(row.try_get(0).map_err(|_| inconsistent())?);
            let number: i64 = row.try_get(1).map_err(|_| inconsistent())?;
            let selected =
                HearingResultRevision::new(u32::try_from(number).map_err(|_| inconsistent())?)
                    .map_err(|_| inconsistent())?;
            crate::hearing_result_postgres::storage::detail(
                &mut tx,
                case,
                hearing,
                result,
                Some(selected),
                &crate::RingSha256Hasher,
            )
            .map_err(|_| inconsistent())?;
            id = Some(result.as_uuid());
            revision = number;
        }
        if rows.len() < 64 {
            return tx.rollback().map_err(port);
        }
    }
}

fn validate_continuations(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let mut complete = HashSet::new();
    let mut after: Option<Uuid> = None;
    loop {
        let rows = tx
            .query(
                "SELECT id FROM case_hearing_results WHERE $1::uuid IS NULL OR id>$1
             ORDER BY id LIMIT 64",
                &[&after],
            )
            .map_err(port)?;
        for row in &rows {
            let id: Uuid = row.try_get(0).map_err(|_| inconsistent())?;
            let mut path = HashSet::new();
            let mut cursor = Some(id);
            while let Some(current) = cursor {
                if complete.contains(&current) {
                    break;
                }
                if !path.insert(current) {
                    return Err(inconsistent());
                }
                let source = tx
                    .query_opt(
                        "SELECT continuation_result_id FROM case_hearing_results WHERE id=$1",
                        &[&current],
                    )
                    .map_err(port)?
                    .ok_or_else(inconsistent)?;
                cursor = source.try_get(0).map_err(|_| inconsistent())?;
            }
            complete.extend(path);
            after = Some(id);
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}

fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "hearing result inventory is inconsistent; restore a consistent database".into(),
    )
}
