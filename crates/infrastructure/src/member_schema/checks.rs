use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> Vec<String> {
    if table != "users" {
        return vec![];
    }
    [
        "(role = ANY (ARRAY['owner'::text, 'litigator'::text, 'paralegal'::text, 'client'::text]))",
        "(revision >= 0)",
        "(auth_generation >= 0)",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client
            .query(
                "SELECT pg_get_expr(c.conbin,c.conrelid),c.convalidated AND NOT c.connoinherit
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.contype='c'",
                &[&table],
            )
            .map_err(port)?;
        let mut actual: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
        let mut expected = expected(table);
        actual.sort();
        expected.sort();
        if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(1)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
