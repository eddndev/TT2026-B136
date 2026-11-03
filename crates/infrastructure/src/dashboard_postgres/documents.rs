use super::port;
use application::ApplicationError;
use postgres::Transaction;
use uuid::Uuid;

/// Contracts of `cases` in `borrador` or `en_firma`: every contract whose
/// last event is neither `completed` nor `abandoned`, whatever the validity
/// of its last request. The free-text document type no longer counts.
/// Rationale: docs/adr/0086-bilateral-contract-signing.md.
pub(super) fn pending_contracts(
    tx: &mut Transaction<'_>,
    cases: &[Uuid],
) -> Result<u64, ApplicationError> {
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM contracts c CROSS JOIN LATERAL (
                SELECT kind FROM contract_events WHERE contract_id=c.id
                ORDER BY revision DESC LIMIT 1) e
            WHERE c.case_id=ANY($1::uuid[]) AND e.kind NOT IN ('completed','abandoned')",
            &[&cases],
        )
        .map_err(port)?
        .get(0);
    u64::try_from(count).map_err(|_| super::inconsistent("negative contract count"))
}
