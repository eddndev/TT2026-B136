use super::{port, CANDIDATES, CURSOR, JOBS, TABLES, TRIGGER_FUNCTIONS};
use application::ApplicationError;
use postgres::GenericClient;

const JOB_COLUMNS: [&str; 8] = [
    "id",
    "operation_id",
    "deadline_id",
    "case_id",
    "event_sequence",
    "bootstrap_policy_version",
    "created_at_seconds",
    "created_at_nanoseconds",
];
const POSITIONS: [&str; 4] = [
    "completed_event_sequence",
    "active_event_sequence",
    "after_deadline_id",
    "bootstrap_after_deadline_id",
];

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let role: String = client
        .query_one("SELECT quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    let job_columns = JOB_COLUMNS.join(",");
    let positions = POSITIONS.join(",");
    let cursor_columns = format!("singleton,{positions}");
    let triggers = TRIGGER_FUNCTIONS.join(",");
    client.batch_execute(&format!(
        "REVOKE ALL ON {JOBS},{CURSOR} FROM {role};
        REVOKE INSERT({job_columns}),UPDATE({job_columns}),REFERENCES({job_columns}) ON {JOBS} FROM {role};
        REVOKE INSERT({cursor_columns}),UPDATE({cursor_columns}),REFERENCES({cursor_columns}) ON {CURSOR} FROM {role};
        GRANT SELECT,INSERT({job_columns}) ON {JOBS} TO {role};
        GRANT SELECT,UPDATE({positions}) ON {CURSOR} TO {role};
        REVOKE ALL ON FUNCTION {triggers},{CANDIDATES} FROM {role};
        GRANT EXECUTE ON FUNCTION {CANDIDATES} TO {role}"
    )).map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let missing: bool = client.query_one(
        "SELECT NOT has_table_privilege($1,$2,'SELECT')
            OR NOT has_table_privilege($1,$3,'SELECT')
            OR EXISTS(SELECT 1 FROM unnest($4::text[]) a WHERE NOT has_column_privilege($1,$2,a,'INSERT'))
            OR EXISTS(SELECT 1 FROM unnest($5::text[]) a WHERE NOT has_column_privilege($1,$3,a,'UPDATE'))
            OR NOT has_function_privilege($1,$6,'EXECUTE')",
        &[&role, &JOBS, &CURSOR, &&JOB_COLUMNS[..], &&POSITIONS[..], &CANDIDATES],
    ).map_err(port)?.get(0);
    let unsafe_role: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM pg_roles r WHERE pg_has_role($1,r.oid,'MEMBER') AND (
            r.rolsuper OR r.rolcreaterole
            OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
                WHERE c.oid IN (SELECT t::regclass FROM unnest($2::text[]) t) AND (
                    pg_has_role(r.oid,c.relowner,'MEMBER') OR pg_has_role(r.oid,n.nspowner,'MEMBER')
                    OR has_schema_privilege(r.oid,n.oid,'CREATE')
                    OR has_table_privilege(r.oid,c.oid,'DELETE,TRUNCATE,TRIGGER,REFERENCES')
                    OR has_any_column_privilege(r.oid,c.oid,'REFERENCES')))
            OR has_table_privilege(r.oid,$3::text,'UPDATE')
            OR has_any_column_privilege(r.oid,$3::text,'UPDATE')
            OR has_table_privilege(r.oid,$4::text,'INSERT,UPDATE')
            OR has_any_column_privilege(r.oid,$4::text,'INSERT')
            OR has_column_privilege(r.oid,$4::text,'singleton','UPDATE')
            OR EXISTS(SELECT 1 FROM pg_proc p
                WHERE p.oid IN (SELECT f::regprocedure FROM unnest($5::text[]||ARRAY[$6::text]) f)
                    AND pg_has_role(r.oid,p.proowner,'MEMBER'))
            OR EXISTS(SELECT 1 FROM unnest($5::text[]) f WHERE has_function_privilege(r.oid,f,'EXECUTE'))))",
        &[&role, &&TABLES[..], &JOBS, &CURSOR, &&TRIGGER_FUNCTIONS[..], &CANDIDATES],
    ).map_err(port)?.get(0);
    let public_privilege: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_class c,
                LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a
            WHERE c.oid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND a.grantee=0)
        OR EXISTS(SELECT 1 FROM pg_attribute c,LATERAL aclexplode(c.attacl) a
            WHERE c.attrelid IN (SELECT t::regclass FROM unnest($1::text[]) t) AND a.grantee=0)
        OR EXISTS(SELECT 1 FROM pg_proc p,
                LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a
            WHERE p.oid IN (SELECT f::regprocedure FROM unnest($2::text[]||ARRAY[$3::text]) f)
                AND a.grantee=0)",
            &[&&TABLES[..], &&TRIGGER_FUNCTIONS[..], &CANDIDATES],
        )
        .map_err(port)?
        .get(0);
    if missing || unsafe_role || public_privilege {
        return Err(ApplicationError::InvalidConfiguration(
            "runtime must only read and append deadline jobs and update protected cursor positions"
                .into(),
        ));
    }
    Ok(())
}
