//! Catalog mutations generated per family from a module specification.
//!
//! A catalog module lists the objects it validates in a `ModuleSpec`; the
//! generator derives tampering for each object family. Equivalence tests apply
//! every mutation with `CatalogFixture::mutated` and compare the verdict of a
//! direct client with the verdict through `PreparedCatalog`, as described in
//! docs/adr/0083-call-scoped-prepared-catalog-statements.md. Templates may use
//! `{runtime}`, `{helper}` and `{schema}`, which the fixture replaces.

mod objects;
mod privileges;

use objects::{function_mutations, index_mutations, redefine, table_mutations, trigger_mutations};
use privileges::privilege_mutations;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Family {
    Table,
    Column,
    Constraint,
    Index,
    Function,
    Trigger,
    Privilege,
}

pub(crate) const FAMILIES: [Family; 7] = [
    Family::Table,
    Family::Column,
    Family::Constraint,
    Family::Index,
    Family::Function,
    Family::Trigger,
    Family::Privilege,
];

pub(crate) struct Mutation {
    pub(crate) family: Family,
    pub(crate) name: String,
    pub(crate) sql: String,
}

/// A column with a default literal of its type and a different type that its
/// current values and dependent objects accept.
pub(crate) struct ColumnSpec {
    pub(crate) table: &'static str,
    pub(crate) column: &'static str,
    pub(crate) default: &'static str,
    pub(crate) retype: &'static str,
}

/// Objects of one catalog module, named as its validator names them.
pub(crate) struct ModuleSpec {
    pub(crate) tables: &'static [&'static str],
    pub(crate) columns: &'static [ColumnSpec],
    /// (table, CHECK constraint)
    pub(crate) checks: &'static [(&'static str, &'static str)],
    /// (table, foreign key constraint)
    pub(crate) foreign_keys: &'static [(&'static str, &'static str)],
    /// (table, index that backs no constraint, ordered key columns)
    pub(crate) indexes: &'static [(&'static str, &'static str, &'static [&'static str])],
    /// Function signatures as accepted by `regprocedure`.
    pub(crate) functions: &'static [&'static str],
    /// (table, trigger)
    pub(crate) triggers: &'static [(&'static str, &'static str)],
    /// Name of a trigger function for an additional trigger.
    pub(crate) trigger_function: &'static str,
    /// (table, column the runtime role must not update)
    pub(crate) protected_columns: &'static [(&'static str, &'static str)],
    /// Function signatures the runtime role must not execute.
    pub(crate) private_functions: &'static [&'static str],
}

pub(crate) fn mutations(spec: &ModuleSpec) -> Vec<Mutation> {
    let mut out = Vec::new();
    let mut add = |family, name: String, sql: String| out.push(Mutation { family, name, sql });
    for table in spec.tables {
        for (name, sql) in table_mutations(table) {
            add(Family::Table, format!("{table} {name}"), sql);
        }
    }
    for column in spec.columns {
        let (table, name) = (column.table, column.column);
        let alter = format!("ALTER TABLE {table} ALTER COLUMN {name}");
        for (kind, sql) in [
            ("nullable", format!("{alter} DROP NOT NULL")),
            ("default", format!("{alter} SET DEFAULT {}", column.default)),
            ("retyped", format!("{alter} TYPE {}", column.retype)),
        ] {
            add(Family::Column, format!("{table}.{name} {kind}"), sql);
        }
    }
    for (table, constraint) in spec.checks {
        let alter = format!("ALTER TABLE {table}");
        for (kind, sql) in [
            (
                "always true",
                format!(
                    "{alter} DROP CONSTRAINT {constraint}, ADD CONSTRAINT {constraint} CHECK(true)"
                ),
            ),
            ("not valid", redefine(table, constraint, "%s NOT VALID")),
            ("no inherit", redefine(table, constraint, "%s NO INHERIT")),
            ("dropped", format!("{alter} DROP CONSTRAINT {constraint}")),
            (
                "extra",
                format!("{alter} ADD CONSTRAINT {table}_extra CHECK(true)"),
            ),
        ] {
            add(Family::Constraint, format!("{constraint} {kind}"), sql);
        }
    }
    for (table, constraint) in spec.foreign_keys {
        let alter = format!("ALTER TABLE {table}");
        for (kind, sql) in [
            (
                "deferrable",
                format!("{alter} ALTER CONSTRAINT {constraint} DEFERRABLE INITIALLY DEFERRED"),
            ),
            ("not valid", redefine(table, constraint, "%s NOT VALID")),
            (
                "cascade",
                redefine(table, constraint, "%s ON DELETE CASCADE"),
            ),
            ("dropped", format!("{alter} DROP CONSTRAINT {constraint}")),
        ] {
            add(Family::Constraint, format!("{constraint} {kind}"), sql);
        }
    }
    for (table, index, columns) in spec.indexes {
        for (kind, sql) in index_mutations(table, index, columns) {
            add(Family::Index, format!("{index} {kind}"), sql);
        }
    }
    for signature in spec.functions {
        for (kind, sql) in function_mutations(signature) {
            add(Family::Function, format!("{signature} {kind}"), sql);
        }
    }
    for (table, trigger) in spec.triggers {
        for (kind, sql) in trigger_mutations(table, trigger, spec.trigger_function) {
            add(Family::Trigger, format!("{trigger} on {table} {kind}"), sql);
        }
    }
    for (name, sql) in privilege_mutations(spec) {
        add(Family::Privilege, name, sql);
    }
    out
}

#[cfg(test)]
mod tests;
