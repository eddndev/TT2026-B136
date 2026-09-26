use application::ApplicationError;
use postgres::Transaction;

const MIGRATION_LOCK_CLASS: i32 = 0x43415345;

// See docs/adr/0048-schema-scoped-migration-lock.md.
pub(super) fn acquire(transaction: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let schema_oid: i32 = transaction
        .query_one(
            "SELECT oid::integer FROM pg_catalog.pg_namespace \
             WHERE nspname=pg_catalog.current_schema()",
            &[],
        )
        .map_err(super::port_error)?
        .get(0);
    transaction
        .query_one(
            "SELECT pg_catalog.pg_advisory_xact_lock($1::integer,$2::integer)",
            &[&MIGRATION_LOCK_CLASS, &schema_oid],
        )
        .map_err(super::port_error)?;
    Ok(())
}
