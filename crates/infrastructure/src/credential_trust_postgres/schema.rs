use application::ApplicationError;
use postgres::GenericClient;

use super::storage::{inconsistent, port};

const AUTHORITY: &str = "participant_credential_authority";
const REVISIONS: &str = "participant_credential_trust_revisions";

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let present: bool = client
        .query_one(
            "SELECT current_setting('server_encoding')='UTF8'
        AND to_regclass('participant_credential_authority') IS NOT NULL
        AND to_regclass('participant_credential_trust_revisions') IS NOT NULL
        AND to_regclass('audit_events') IS NOT NULL",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !present {
        return Err(incomplete());
    }
    columns(
        client,
        AUTHORITY,
        &[
            ("deployment_id", "uuid"),
            ("singleton", "boolean"),
            ("root_der", "bytea"),
            ("root_fingerprint", "bytea"),
            ("initial_revision", "bigint"),
        ],
    )?;
    columns(
        client,
        REVISIONS,
        &[
            ("deployment_id", "uuid"),
            ("revision", "bigint"),
            ("crl_der", "bytea"),
            ("crl_digest", "bytea"),
            ("crl_number", "numeric(20,0)"),
            ("crl_this_update", "bigint"),
            ("crl_next_update", "bigint"),
            ("valid_from", "bigint"),
            ("valid_until", "bigint"),
            ("published_at_seconds", "bigint"),
            ("published_at_nanoseconds", "integer"),
            ("published_by", "text"),
        ],
    )?;
    key(client, AUTHORITY, "p", &["deployment_id"])?;
    key(client, AUTHORITY, "u", &["singleton"])?;
    key(client, REVISIONS, "p", &["deployment_id", "revision"])?;
    for (table, names) in [
        (
            AUTHORITY,
            vec![
                "credential_authority_singleton",
                "credential_authority_initial",
                "credential_authority_material",
                "credential_authority_digest",
            ],
        ),
        (
            REVISIONS,
            vec![
                "credential_trust_revision_range",
                "credential_trust_material",
                "credential_trust_digest",
                "credential_trust_number_range",
                "credential_trust_window",
                "credential_trust_published_time",
                "credential_trust_published_by",
            ],
        ),
    ] {
        let valid: bool = client.query_one("SELECT count(*)=cardinality($2::text[]) FROM pg_catalog.pg_constraint
            WHERE conrelid=$1::text::regclass AND contype='c' AND convalidated AND conname::text=ANY($2::text[])",
            &[&table,&names]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    foreign_key(
        client,
        AUTHORITY,
        "credential_authority_initial_fk",
        REVISIONS,
        &["deployment_id", "initial_revision"],
        &["deployment_id", "revision"],
        true,
    )?;
    foreign_key(
        client,
        REVISIONS,
        "credential_trust_root_fk",
        AUTHORITY,
        &["deployment_id"],
        &["deployment_id"],
        false,
    )?;
    for function in [
        "preserve_credential_trust_history()",
        "enforce_credential_trust_sequence()",
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_proc
            WHERE oid=to_regprocedure($1) AND NOT prosecdef AND provolatile='v' AND prorettype='trigger'::regtype)",
            &[&function]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, name, kind, function) in [
        (
            AUTHORITY,
            "credential_authority_immutable",
            27_i16,
            "preserve_credential_trust_history()",
        ),
        (
            AUTHORITY,
            "credential_authority_no_truncate",
            34,
            "preserve_credential_trust_history()",
        ),
        (
            REVISIONS,
            "credential_trust_immutable",
            27,
            "preserve_credential_trust_history()",
        ),
        (
            REVISIONS,
            "credential_trust_no_truncate",
            34,
            "preserve_credential_trust_history()",
        ),
        (
            REVISIONS,
            "credential_trust_sequence",
            7,
            "enforce_credential_trust_sequence()",
        ),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger
            WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgtype=$3 AND tgfoid=to_regprocedure($4)
                AND tgenabled IN ('O','A') AND NOT tgdeferrable AND NOT tginitdeferred)",
            &[&table,&name,&kind,&function]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let enabled: bool = client
        .query_one(
            "SELECT NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t
        JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint
        WHERE c.conrelid IN ('participant_credential_authority'::regclass,
            'participant_credential_trust_revisions'::regclass) AND t.tgenabled NOT IN ('O','A'))",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !enabled {
        return Err(incomplete());
    }
    Ok(())
}

fn columns<C: GenericClient>(
    client: &mut C,
    table: &str,
    expected: &[(&str, &str)],
) -> Result<(), ApplicationError> {
    let rows = client.query("SELECT a.attname,pg_catalog.format_type(a.atttypid,a.atttypmod),a.attnotnull
        FROM pg_catalog.pg_attribute a JOIN pg_catalog.pg_class c ON c.oid=a.attrelid
        WHERE a.attrelid=$1::text::regclass AND c.relkind='r' AND a.attnum>0 AND NOT a.attisdropped",&[&table])
        .map_err(port)?;
    if rows.len() != expected.len()
        || expected.iter().any(|(name, kind)| {
            !rows.iter().any(|row| {
                row.get::<_, String>(0) == *name
                    && row.get::<_, String>(1) == *kind
                    && row.get::<_, bool>(2)
            })
        })
    {
        return Err(incomplete());
    }
    Ok(())
}
fn key<C: GenericClient>(
    client: &mut C,
    table: &str,
    kind: &str,
    columns: &[&str],
) -> Result<(), ApplicationError> {
    let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
        WHERE c.conrelid=$1::text::regclass AND c.contype::text=$2 AND c.convalidated
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
            JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$3::text[])",
        &[&table,&kind,&columns]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
fn foreign_key<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    target: &str,
    columns: &[&str],
    targets: &[&str],
    deferred: bool,
) -> Result<(), ApplicationError> {
    let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
        WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype='f' AND c.convalidated
        AND c.confrelid=$3::text::regclass AND c.condeferrable=$6 AND c.condeferred=$6
        AND c.confdeltype='a' AND c.confupdtype='a'
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
            JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$4::text[]
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
            JOIN pg_catalog.pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$5::text[])",
        &[&table,&name,&target,&columns,&targets,&deferred]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let broken:bool=client.query_one("SELECT
        (SELECT count(*) FROM participant_credential_authority)>1
        OR EXISTS(SELECT 1 FROM participant_credential_authority a
            LEFT JOIN (SELECT deployment_id,min(revision) first,max(revision) last,count(*) total
                FROM participant_credential_trust_revisions GROUP BY deployment_id) h USING(deployment_id)
            WHERE NOT a.singleton OR a.initial_revision<>1 OR h.first IS DISTINCT FROM 1 OR h.total<>h.last
                OR octet_length(a.root_der) NOT BETWEEN 1 AND 16384 OR a.root_fingerprint<>pg_catalog.sha256(a.root_der))
        OR EXISTS(SELECT 1 FROM (SELECT r.*,lag(crl_number) OVER w previous_number,
                lag(crl_this_update) OVER w previous_update FROM participant_credential_trust_revisions r
                WINDOW w AS (PARTITION BY deployment_id ORDER BY revision)) r
            LEFT JOIN participant_credential_authority a USING(deployment_id)
            WHERE a.deployment_id IS NULL OR r.revision NOT BETWEEN 1 AND 4294967295
                OR octet_length(r.crl_der) NOT BETWEEN 1 AND 1048576 OR r.crl_digest<>pg_catalog.sha256(r.crl_der)
                OR r.crl_number NOT BETWEEN 0 AND 18446744073709551615
                OR r.crl_number<=r.previous_number OR r.crl_this_update<r.previous_update
                OR r.crl_this_update<0 OR r.crl_next_update<=r.crl_this_update OR r.crl_next_update>253402300799
                OR r.valid_from<r.crl_this_update OR r.valid_until>r.crl_next_update OR r.valid_from>r.valid_until
                OR r.published_at_seconds NOT BETWEEN r.valid_from AND r.valid_until
                OR r.published_at_nanoseconds NOT BETWEEN 0 AND 999999999
                OR octet_length(r.published_by) NOT BETWEEN 1 AND 1024)",&[]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    Ok(())
}

pub(crate) fn grant_runtime<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let quoted: String = client
        .query_one("SELECT pg_catalog.quote_ident($1)", &[&role])
        .map_err(port)?
        .get(0);
    client.batch_execute(&format!("REVOKE ALL ON participant_credential_authority,participant_credential_trust_revisions FROM {quoted};
        GRANT SELECT ON participant_credential_authority,participant_credential_trust_revisions TO {quoted};
        REVOKE ALL ON FUNCTION preserve_credential_trust_history(),enforce_credential_trust_sequence() FROM {quoted};"))
        .map_err(port)
}

pub(crate) fn validate_runtime_role<C: GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    let unsafe_role:bool=client.query_one("SELECT
        NOT pg_catalog.has_table_privilege($1,'participant_credential_authority','SELECT')
        OR NOT pg_catalog.has_table_privilege($1,'participant_credential_trust_revisions','SELECT')
        OR EXISTS(SELECT 1 FROM pg_catalog.pg_roles r WHERE pg_catalog.pg_has_role($1,r.oid,'MEMBER')
            AND (r.rolsuper OR r.rolcreaterole OR EXISTS(
                SELECT 1 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
                WHERE c.oid=ANY(ARRAY['participant_credential_authority'::regclass,
                    'participant_credential_trust_revisions'::regclass])
                AND (pg_catalog.pg_has_role(r.oid,c.relowner,'MEMBER')
                    OR pg_catalog.pg_has_role(r.oid,n.nspowner,'MEMBER')
                    OR pg_catalog.has_table_privilege(r.oid,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER')
                    OR pg_catalog.has_any_column_privilege(r.oid,c.oid,'INSERT,UPDATE')))
            OR EXISTS(SELECT 1 FROM pg_catalog.pg_proc p WHERE p.oid=ANY(ARRAY[
                    'preserve_credential_trust_history()'::regprocedure,
                    'enforce_credential_trust_sequence()'::regprocedure])
                AND (pg_catalog.pg_has_role(r.oid,p.proowner,'MEMBER')
                    OR pg_catalog.has_function_privilege(r.oid,p.oid,'EXECUTE')))))", &[&role])
        .map_err(port)?.get(0);
    if unsafe_role {
        return Err(ApplicationError::InvalidConfiguration(
        "runtime database role must only read credential trust and must not own its protected objects".into()));
    }
    Ok(())
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "credential trust schema is incomplete; run database migrate with an administrative role"
            .into(),
    )
}
