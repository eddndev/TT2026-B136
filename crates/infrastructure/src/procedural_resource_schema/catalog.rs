use super::{columns, constraints, functions, incomplete, indexes, port, triggers, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_class c
            JOIN pg_class d ON d.oid='case_deadlines'::regclass
            WHERE c.oid=to_regclass($1) AND c.relnamespace=d.relnamespace AND c.relowner=d.relowner
                AND c.relkind='r' AND c.relpersistence='p' AND NOT c.relispartition
                AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity
                AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=c.oid OR inhparent=c.oid)
                AND NOT EXISTS(SELECT 1 FROM pg_rewrite WHERE ev_class=c.oid))",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    columns::validate(client)?;
    constraints::validate(client)?;
    indexes::validate(client)?;
    functions::validate(client)?;
    triggers::validate(client)
}
