use super::{columns, constraints, functions, incomplete, indexes, port, triggers, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate_schema<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_class c JOIN pg_class u ON u.oid='users'::regclass
        WHERE c.oid=to_regclass($1) AND c.relnamespace=u.relnamespace AND c.relowner=u.relowner
            AND c.relkind='r' AND c.relpersistence='p' AND NOT c.relispartition
            AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity
            AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=c.oid OR inhparent=c.oid)
            AND NOT EXISTS(SELECT 1 FROM pg_rewrite WHERE ev_class=c.oid))",
            &[&TABLE],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    columns::validate(client).map_err(|error| component(error, "columns"))?;
    constraints::validate(client).map_err(|error| component(error, "constraints"))?;
    indexes::validate(client).map_err(|error| component(error, "indexes"))?;
    functions::validate(client).map_err(|error| component(error, "functions"))?;
    triggers::validate(client).map_err(|error| component(error, "triggers"))
}

fn component(error: ApplicationError, name: &str) -> ApplicationError {
    match error {
        ApplicationError::InvalidConfiguration(_) => ApplicationError::InvalidConfiguration(
            format!("password reset {name} catalog is incomplete or altered"),
        ),
        ApplicationError::Port(_) => {
            ApplicationError::Port(format!("password reset {name} catalog query failed"))
        }
        other => other,
    }
}
