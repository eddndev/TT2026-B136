use application::ApplicationError;
use postgres::GenericClient;

use super::{invalid, port};

type Constraint = (&'static str, &'static [&'static str], Option<&'static str>);

const USERS: &[Constraint] = &[
    ("p", &["id"], None),
    ("u", &["email"], None),
    ("c", &["role"], Some("(role = 'client'::text)")),
    ("c", &["revision"], Some("(revision >= 0)")),
    ("c", &["auth_generation"], Some("(auth_generation >= 0)")),
];
const AUDIT: &[Constraint] = &[
    ("p", &["sequence"], None),
    ("c", &["sequence"], Some("(sequence >= 0)")),
    ("c", &["chain"], Some("(octet_length(chain) = 32)")),
    (
        "c",
        &["context_json"],
        Some("(octet_length(context_json) <= 1024)"),
    ),
];

pub(super) fn validate(
    client: &mut impl GenericClient,
    table: &str,
) -> Result<(), ApplicationError> {
    let expected = if table == "users" { USERS } else { AUDIT };
    let rows = client.query(
        "SELECT c.contype::text,
         ARRAY(SELECT a.attname::text FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
             JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num ORDER BY k.n),
         pg_catalog.pg_get_expr(c.conbin,c.conrelid),
         c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
         AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
         AND c.connamespace=t.relnamespace
         AND (pg_catalog.to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
         AND (c.contype='c' OR EXISTS(
             SELECT 1 FROM pg_catalog.pg_index i JOIN pg_catalog.pg_class x ON x.oid=i.indexrelid
             WHERE i.indexrelid=c.conindid AND i.indrelid=c.conrelid AND i.indisunique
                 AND i.indisvalid AND i.indisready AND i.indislive AND i.indimmediate
                 AND i.indisprimary=(c.contype='p') AND i.indpred IS NULL AND i.indexprs IS NULL
                 AND i.indnatts=i.indnkeyatts AND i.indnkeyatts=cardinality(c.conkey)
                 AND ARRAY(SELECT pg_catalog.unnest(i.indkey))=c.conkey
                 AND x.relnamespace=t.relnamespace))
         FROM pg_catalog.pg_constraint c JOIN pg_catalog.pg_class t ON t.oid=c.conrelid
         WHERE c.conrelid=pg_catalog.to_regclass('qadra_ca.' || $1) AND c.contype<>'n'",
        &[&table],
    ).map_err(port)?;
    let mut seen = vec![false; expected.len()];
    for row in &rows {
        let kind: String = row.get(0);
        let columns: Vec<String> = row.get(1);
        let expression: Option<String> = row.get(2);
        let found = expected.iter().position(|value| {
            kind == value.0
                && columns
                    .iter()
                    .map(String::as_str)
                    .eq(value.1.iter().copied())
                && expression.as_deref() == value.2
        });
        let Some(index) = found else {
            return Err(invalid("CA identity constraint differs"));
        };
        if seen[index] || !row.get::<_, bool>(3) {
            return Err(invalid("CA identity constraint is duplicated or unsafe"));
        }
        seen[index] = true;
    }
    let indexes: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_catalog.pg_index
         WHERE indrelid=pg_catalog.to_regclass('qadra_ca.' || $1)",
            &[&table],
        )
        .map_err(port)?
        .get(0);
    let expected_indexes = if table == "users" { 2 } else { 1 };
    if seen.iter().any(|present| !present) || indexes != expected_indexes {
        return Err(invalid("CA identity constraints or indexes are incomplete"));
    }
    Ok(())
}
