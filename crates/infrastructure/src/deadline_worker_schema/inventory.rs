//! Replay every immutable ledger row using the same readers as the worker port.
use super::{incomplete, port, reserved, ATTEMPTS, RESULTS};
use application::ApplicationError;
use domain::typed_participants::Uuid;
use postgres::{Client, Transaction};

enum Ledger {
    Jobs,
    Results,
    Attempts,
}

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let gaps: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM deadline_reevaluation_attempts GROUP BY job_id
            HAVING min(attempt_number)<>1 OR max(attempt_number)<>count(*))",
            &[],
        )
        .map_err(port)?
        .get(0);
    if gaps {
        return Err(incomplete());
    }
    for (table, key, ledger) in [
        ("deadline_reevaluation_jobs", "id", Ledger::Jobs),
        (RESULTS, "job_id", Ledger::Results),
        (ATTEMPTS, "attempt_id", Ledger::Attempts),
    ] {
        rows(&mut tx, table, key, ledger)?;
    }
    reserved::validate(&mut tx)?;
    tx.rollback().map_err(port)
}

fn rows(
    tx: &mut Transaction<'_>,
    table: &str,
    key: &str,
    ledger: Ledger,
) -> Result<(), ApplicationError> {
    // Table and key come only from the fixed private list above. The query
    // materializes identifiers; each actual reader bounds variable-size data.
    let query = format!(
        "SELECT {key} FROM {table}
        WHERE $1::uuid IS NULL OR {key}>$1 ORDER BY {key} LIMIT 64"
    );
    let mut after: Option<Uuid> = None;
    loop {
        let page = tx.query(&query, &[&after]).map_err(port)?;
        for row in &page {
            let id: Uuid = row.try_get(0).map_err(|_| incomplete())?;
            match ledger {
                Ledger::Jobs => {
                    crate::deadline_worker_provenance::load_job(tx, id, &crate::RingSha256Hasher)?;
                }
                Ledger::Results => {
                    crate::deadline_worker_postgres::results::load(
                        tx,
                        id,
                        &crate::RingSha256Hasher,
                    )?
                    .ok_or_else(incomplete)?;
                }
                Ledger::Attempts => {
                    crate::deadline_worker_postgres::attempts_read::by_id(
                        tx,
                        id,
                        &crate::RingSha256Hasher,
                    )?
                    .ok_or_else(incomplete)?;
                }
            }
            after = Some(id);
        }
        if page.len() < 64 {
            return Ok(());
        }
    }
}
