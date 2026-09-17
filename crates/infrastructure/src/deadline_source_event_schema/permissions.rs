use super::{port, FUNCTIONS, SEQUENCE, TABLE};
use application::ApplicationError;
use postgres::GenericClient;
const INPUTS: [&str; 6] = [
    "source_kind",
    "source_id",
    "revision",
    "case_id",
    "hearing_id",
    "operation_id",
];
const DERIVED: [&str; 5] = [
    "sequence",
    "fact_family",
    "hearing_result_id",
    "calendar_id",
    "profile_id",
];

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let role: String = client
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    client
        .batch_execute(&format!(
            "REVOKE ALL ON {TABLE} FROM {role};
        REVOKE INSERT({}),UPDATE({}) ON {TABLE} FROM {role};
        GRANT SELECT,INSERT({}) ON {TABLE} TO {role};
        REVOKE ALL ON SEQUENCE {SEQUENCE} FROM {role}; GRANT USAGE ON SEQUENCE {SEQUENCE} TO {role};
        REVOKE ALL ON FUNCTION {} FROM {role}",
            INPUTS
                .into_iter()
                .chain(DERIVED)
                .collect::<Vec<_>>()
                .join(","),
            INPUTS
                .into_iter()
                .chain(DERIVED)
                .collect::<Vec<_>>()
                .join(","),
            INPUTS.join(","),
            FUNCTIONS.join(",")
        ))
        .map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let invalid: bool = client.query_one("SELECT NOT has_table_privilege($1,$2,'SELECT')
        OR NOT has_sequence_privilege($1,$3,'USAGE')
        OR EXISTS(SELECT 1 FROM unnest($4::text[]) a WHERE NOT has_column_privilege($1,$2,a,'INSERT'))
        OR EXISTS(SELECT 1 FROM unnest($5::text[]) a WHERE has_column_privilege($1,$2,a,'INSERT'))
        OR EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND (
            r.rolsuper OR r.rolcreaterole
            OR has_table_privilege(r.oid,$2::text,'UPDATE,DELETE,TRUNCATE,TRIGGER')
            OR has_any_column_privilege(r.oid,$2::text,'UPDATE')
            OR has_sequence_privilege(r.oid,$3::text,'UPDATE')
            OR EXISTS(SELECT 1 FROM pg_class c WHERE c.oid IN ($2::text::regclass,$3::text::regclass)
                AND pg_has_role(r.oid,c.relowner,'MEMBER'))
            OR EXISTS(SELECT 1 FROM pg_proc p WHERE p.oid IN (SELECT f::regprocedure FROM unnest($6::text[]) f)
                AND (pg_has_role(r.oid,p.proowner,'MEMBER') OR has_function_privilege(r.oid,p.oid,'EXECUTE')))))",
        &[&role,&TABLE,&SEQUENCE,&&INPUTS[..],&&DERIVED[..],&&FUNCTIONS[..]]).map_err(port)?.get(0);
    if invalid {
        return Err(ApplicationError::InvalidConfiguration(
        "runtime role must only read and append deadline source events with protected sequence allocation".into()));
    }
    Ok(())
}
