//! Client boundary for catalog and runtime role validation.
//!
//! Validation code issues SQL text through `CatalogClient`. A direct client or
//! transaction delegates to `postgres::GenericClient`, which prepares, executes
//! and closes a statement for every call. `PreparedCatalog` reuses one prepared
//! statement per exact SQL text for the lifetime of a single validation call.
//! It never stores results. Rationale:
//! docs/adr/0083-call-scoped-prepared-catalog-statements.md.

use postgres::types::ToSql;
use postgres::{Client, Error, GenericClient, Row, Statement, Transaction};
use std::collections::HashMap;

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

/// Reuses prepared statements by exact SQL text during one validation call.
///
/// The statements are protocol state of the borrowed connection. Dropping this
/// value drops them, and the client then closes them on that connection before
/// its next request. Nothing is shared with other connections or calls.
pub(crate) struct PreparedCatalog<'c> {
    client: &'c mut Client,
    statements: HashMap<Box<str>, Statement>,
    #[cfg(test)]
    prepares: usize,
}

impl<'c> PreparedCatalog<'c> {
    pub(crate) fn new(client: &'c mut Client) -> Self {
        Self {
            client,
            statements: HashMap::new(),
            #[cfg(test)]
            prepares: 0,
        }
    }

    /// Returns the statement for this exact text, preparing it on first use.
    /// A failed preparation is returned unchanged and not remembered.
    fn statement(&mut self, sql: &str) -> Result<Statement, Error> {
        if let Some(statement) = self.statements.get(sql) {
            return Ok(statement.clone());
        }
        #[cfg(test)]
        {
            self.prepares += 1;
        }
        let statement = self.client.prepare(sql)?;
        self.statements.insert(Box::from(sql), statement.clone());
        Ok(statement)
    }

    #[cfg(test)]
    fn prepares(&self) -> usize {
        self.prepares
    }
}

impl CatalogClient for PreparedCatalog<'_> {
    fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        let statement = self.statement(sql)?;
        self.client.query(&statement, params)
    }
    fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        let statement = self.statement(sql)?;
        self.client.query_one(&statement, params)
    }
    fn query_opt(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        let statement = self.statement(sql)?;
        self.client.query_opt(&statement, params)
    }
}

#[cfg(test)]
mod tests;
