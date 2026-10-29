//! Exact catalog of the sealed evidence manifests of individual signatures
//! and of the cutoff after which every receipt has one. Rationale: the
//! section "Generation inside the acceptance operation" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.
mod checks;
mod columns;
mod constraints;
mod functions;
mod indexes;
mod inventory;
mod specifications;
mod triggers;

use crate::postgres::CatalogClient;
use crate::signing_binding_schema::{catalog, permissions};
use application::ApplicationError;
pub(crate) use inventory::validate_inventory;
use postgres::GenericClient;

pub(crate) const MIGRATIONS: &[&str] = &[include_str!(
    "../../../../migrations/0049_individual_signing_manifests.sql"
)];
const TABLES: [&str; 2] = [
    "individual_document_manifests",
    "individual_manifest_cutoff",
];
/// The receipt table, whose namespace and owner the functions share.
const RECEIPTS: &str = "individual_document_signatures";

pub(crate) fn validate_schema<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    catalog::validate_tables(client, &TABLES, incomplete, port)?;
    columns::validate(client)?;
    constraints::validate(client)?;
    indexes::validate(client)?;
    functions::validate(client)?;
    triggers::validate(client)
}

/// The runtime reads both tables and appends manifests; the cutoff is
/// written only by the migration.
pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    permissions::grant_runtime(
        client,
        role,
        &TABLES[..1],
        columns::names,
        &functions::TRIGGERS,
        port,
    )?;
    let quoted: String = client
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    let names = columns::names(TABLES[1]).join(",");
    client
        .batch_execute(&format!(
            "REVOKE ALL ON {table} FROM {quoted};
            REVOKE SELECT({names}),INSERT({names}),UPDATE({names}),REFERENCES({names})
                ON {table} FROM {quoted};
            GRANT SELECT ON {table} TO {quoted}",
            table = TABLES[1]
        ))
        .map_err(port)
}

pub(crate) fn validate_runtime<C: CatalogClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    permissions::validate_runtime(
        client,
        role,
        &TABLES,
        columns::insertable,
        &functions::TRIGGERS,
        port,
    )?;
    let writes: bool = client
        .query_one(
            "SELECT has_any_column_privilege($1,$2,'INSERT')",
            &[&role, &TABLES[1]],
        )
        .map_err(port)?
        .get(0);
    if writes {
        return Err(ApplicationError::InvalidConfiguration(
            "runtime must not write the individual manifest cutoff".into(),
        ));
    }
    Ok(())
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "individual signing manifest schema is incomplete or altered".into(),
    )
}

fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("individual signing manifest schema operation failed".into())
}
