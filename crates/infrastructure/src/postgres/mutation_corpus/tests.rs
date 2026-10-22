use super::{mutations, ColumnSpec, Family, ModuleSpec, FAMILIES};
use std::collections::BTreeSet;

const SPEC: ModuleSpec = ModuleSpec {
    tables: &["sample_rows"],
    columns: &[ColumnSpec {
        table: "sample_rows",
        column: "amount",
        default: "0",
        retype: "numeric",
    }],
    checks: &[("sample_rows", "sample_rows_amount_check")],
    foreign_keys: &[("sample_rows", "sample_rows_owner_fkey")],
    indexes: &[("sample_rows", "sample_rows_order", &["owner", "amount"])],
    functions: &["sample_valid(bigint)"],
    triggers: &[("sample_rows", "sample_lock")],
    trigger_function: "lock_sample_rows",
    protected_columns: &[("sample_rows", "amount")],
    private_functions: &["lock_sample_rows()"],
};

#[test]
fn generator_covers_every_family_with_unique_names() {
    let corpus = mutations(&SPEC);
    let families: BTreeSet<Family> = corpus.iter().map(|mutation| mutation.family).collect();
    assert_eq!(families, BTreeSet::from(FAMILIES));
    let names: BTreeSet<&str> = corpus
        .iter()
        .map(|mutation| mutation.name.as_str())
        .collect();
    assert_eq!(names.len(), corpus.len(), "duplicate mutation names");
    for family in FAMILIES {
        let count = corpus.iter().filter(|m| m.family == family).count();
        assert!(count >= 3, "{family:?} has only {count} mutations");
    }
}

#[test]
fn privilege_mutations_cover_public_grant_options_and_membership() {
    let corpus = mutations(&SPEC);
    let privileges: Vec<&str> = corpus
        .iter()
        .filter(|mutation| mutation.family == Family::Privilege)
        .map(|mutation| mutation.sql.as_str())
        .collect();
    for needle in [
        "TO PUBLIC",
        "WITH GRANT OPTION",
        "ALTER ROLE {runtime} NOINHERIT",
        "WITH INHERIT FALSE, SET TRUE",
        "GRANT EXECUTE ON FUNCTION lock_sample_rows()",
        "GRANT UPDATE(amount)",
    ] {
        assert!(
            privileges.iter().any(|sql| sql.contains(needle)),
            "no privilege mutation contains {needle}"
        );
    }
}

#[test]
fn templates_use_only_fixture_placeholders() {
    for mutation in mutations(&SPEC) {
        let rest = mutation
            .sql
            .replace("{runtime}", "")
            .replace("{helper}", "")
            .replace("{schema}", "");
        assert!(
            !rest.contains('{') && !rest.contains('}'),
            "{} keeps an unknown placeholder: {}",
            mutation.name,
            mutation.sql
        );
        assert!(mutation.sql.is_ascii());
    }
}

#[test]
fn index_mutations_reorder_and_include_only_known_key_columns() {
    let corpus = mutations(&SPEC);
    let sql = |name: &str| {
        corpus
            .iter()
            .find(|mutation| mutation.name == name)
            .map(|mutation| mutation.sql.clone())
            .unwrap()
    };
    assert!(sql("sample_rows_order descending").ends_with("sample_rows(owner DESC,amount)"));
    assert!(sql("sample_rows_order included").ends_with("sample_rows(owner) INCLUDE(amount)"));
}
