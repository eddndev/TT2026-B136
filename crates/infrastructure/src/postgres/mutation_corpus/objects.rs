//! Tampering of tables, constraints, indexes, functions and triggers.

pub(super) fn table_mutations(table: &str) -> [(&'static str, String); 6] {
    let alter = format!("ALTER TABLE {table}");
    [
        ("unlogged", format!("{alter} SET UNLOGGED")),
        ("row security", format!("{alter} ENABLE ROW LEVEL SECURITY")),
        (
            "forced row security",
            format!("{alter} FORCE ROW LEVEL SECURITY"),
        ),
        (
            "rule",
            format!("CREATE RULE {table}_extra AS ON DELETE TO {table} DO INSTEAD NOTHING"),
        ),
        (
            "child",
            format!("CREATE TABLE {table}_child () INHERITS ({table})"),
        ),
        (
            "extra column",
            format!("{alter} ADD COLUMN unexpected text"),
        ),
    ]
}

/// Drops a constraint and adds it again from its own definition, formatted
/// into `pattern` at `%s`.
pub(super) fn redefine(table: &str, constraint: &str, pattern: &str) -> String {
    format!(
        "DO $mutation$ BEGIN EXECUTE format(
            'ALTER TABLE {table} DROP CONSTRAINT {constraint}, ADD CONSTRAINT {constraint} {pattern}',
            (SELECT pg_catalog.pg_get_constraintdef(oid) FROM pg_catalog.pg_constraint
                WHERE conrelid OPERATOR(pg_catalog.=) '{table}'::regclass
                    AND conname OPERATOR(pg_catalog.=) '{constraint}'));
        END $mutation$"
    )
}

pub(super) fn index_mutations(
    table: &str,
    index: &str,
    columns: &[&str],
) -> Vec<(&'static str, String)> {
    let drop = format!("DROP INDEX {index};");
    let keys = columns.join(",");
    let mut descending = columns.iter().map(|c| c.to_string()).collect::<Vec<_>>();
    descending[0].push_str(" DESC");
    let mut out = vec![
        ("dropped", format!("DROP INDEX {index}")),
        (
            "descending",
            format!(
                "{drop} CREATE INDEX {index} ON {table}({})",
                descending.join(",")
            ),
        ),
        (
            "partial",
            format!(
                "{drop} CREATE INDEX {index} ON {table}({keys}) WHERE {} IS NOT NULL",
                columns[0]
            ),
        ),
        (
            "unique",
            format!("{drop} CREATE UNIQUE INDEX {index} ON {table}({keys})"),
        ),
        (
            "hash",
            format!(
                "{drop} CREATE INDEX {index} ON {table} USING hash({})",
                columns[0]
            ),
        ),
        (
            "extra",
            format!("CREATE INDEX {index}_extra ON {table}({keys})"),
        ),
    ];
    if let Some((last, leading)) = columns.split_last().filter(|(_, rest)| !rest.is_empty()) {
        out.push((
            "included",
            format!(
                "{drop} CREATE INDEX {index} ON {table}({}) INCLUDE({last})",
                leading.join(",")
            ),
        ));
    }
    out
}

/// Each attribute change flips the current value, so it always alters the catalog.
pub(super) fn function_mutations(signature: &str) -> [(&'static str, String); 8] {
    let alter = format!("ALTER FUNCTION {signature}");
    let flip = |column: &str, current: &str, changed: &str, otherwise: &str| {
        format!(
            "DO $mutation$ BEGIN EXECUTE CASE WHEN (SELECT {column} FROM pg_catalog.pg_proc
                WHERE oid OPERATOR(pg_catalog.=) '{signature}'::regprocedure)::text
                OPERATOR(pg_catalog.=) '{current}'
                THEN '{alter} {changed}' ELSE '{alter} {otherwise}' END; END $mutation$"
        )
    };
    [
        ("volatility", flip("provolatile", "v", "STABLE", "VOLATILE")),
        (
            "strictness",
            flip("proisstrict", "true", "CALLED ON NULL INPUT", "STRICT"),
        ),
        (
            "parallel",
            flip("proparallel", "s", "PARALLEL UNSAFE", "PARALLEL SAFE"),
        ),
        (
            "leakproof",
            flip("proleakproof", "true", "NOT LEAKPROOF", "LEAKPROOF"),
        ),
        ("security definer", format!("{alter} SECURITY DEFINER")),
        (
            "configuration",
            format!("{alter} SET search_path = pg_temp"),
        ),
        (
            "owner",
            format!("CREATE ROLE {{helper}} NOLOGIN; {alter} OWNER TO {{helper}}"),
        ),
        (
            "body",
            format!(
                "DO $mutation$ BEGIN EXECUTE pg_catalog.regexp_replace(
                    pg_catalog.pg_get_functiondef('{signature}'::regprocedure),
                    'AS \\$function\\$', 'AS $function$ ');
                END $mutation$"
            ),
        ),
    ]
}

pub(super) fn trigger_mutations(
    table: &str,
    trigger: &str,
    function: &str,
) -> [(&'static str, String); 6] {
    let alter = format!("ALTER TABLE {table}");
    [
        ("disabled", format!("{alter} DISABLE TRIGGER {trigger}")),
        (
            "replica",
            format!("{alter} ENABLE REPLICA TRIGGER {trigger}"),
        ),
        ("dropped", format!("DROP TRIGGER {trigger} ON {table}")),
        (
            "conditional",
            recreate(
                table,
                trigger,
                "pg_catalog.replace(definition, ' EXECUTE ', ' WHEN (false) EXECUTE ')",
            ),
        ),
        (
            "level",
            recreate(
                table,
                trigger,
                "CASE WHEN pg_catalog.strpos(definition, 'FOR EACH ROW') OPERATOR(pg_catalog.>) 0
                    THEN pg_catalog.replace(definition, 'FOR EACH ROW', 'FOR EACH STATEMENT')
                    ELSE pg_catalog.replace(definition, 'FOR EACH STATEMENT', 'FOR EACH ROW') END",
            ),
        ),
        (
            "extra",
            format!(
                "CREATE TRIGGER {trigger}_extra BEFORE INSERT ON {table}
                    FOR EACH ROW EXECUTE FUNCTION {function}()"
            ),
        ),
    ]
}

/// Drops a trigger and creates it again from `definition`, its own
/// definition, rewritten by the SQL expression `rewrite`.
fn recreate(table: &str, trigger: &str, rewrite: &str) -> String {
    format!(
        "DO $mutation$ DECLARE definition text := pg_catalog.pg_get_triggerdef(
            (SELECT oid FROM pg_catalog.pg_trigger
                WHERE tgrelid OPERATOR(pg_catalog.=) '{table}'::regclass
                    AND tgname OPERATOR(pg_catalog.=) '{trigger}'));
        BEGIN EXECUTE 'DROP TRIGGER {trigger} ON {table}'; EXECUTE {rewrite}; END $mutation$"
    )
}
