//! Client boundary for catalog and runtime role validation.
//!
//! Validation code issues SQL text through `CatalogClient`. A direct client or
//! transaction delegates to `postgres::GenericClient`, which prepares, executes
//! and closes a statement for every call. Rationale:
//! docs/adr/0083-call-scoped-prepared-catalog-statements.md.

use postgres::types::ToSql;
use postgres::{Client, Error, GenericClient, Row, Transaction};

/// Query methods used by catalog and runtime role validation, with the
/// semantics of the `postgres::GenericClient` methods of the same name for SQL
/// text. Validation issues no `execute`, `batch_execute` or transaction calls.
pub(crate) trait CatalogClient {
    fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>;
    fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error>;
    fn query_opt(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>;
}

macro_rules! direct_catalog_client {
    ($client:ty) => {
        impl CatalogClient for $client {
            fn query(
                &mut self,
                sql: &str,
                params: &[&(dyn ToSql + Sync)],
            ) -> Result<Vec<Row>, Error> {
                GenericClient::query(self, sql, params)
            }
            fn query_one(
                &mut self,
                sql: &str,
                params: &[&(dyn ToSql + Sync)],
            ) -> Result<Row, Error> {
                GenericClient::query_one(self, sql, params)
            }
            fn query_opt(
                &mut self,
                sql: &str,
                params: &[&(dyn ToSql + Sync)],
            ) -> Result<Option<Row>, Error> {
                GenericClient::query_opt(self, sql, params)
            }
        }
    };
}

direct_catalog_client!(Client);
direct_catalog_client!(Transaction<'_>);
