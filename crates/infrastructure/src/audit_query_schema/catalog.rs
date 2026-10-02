use super::{functions, incomplete, port};
use application::ApplicationError;
use postgres::GenericClient;

const COLUMNS: &[(&str, &str, &str)] = &[
    ("sequence", "bigint", ""),
    ("timestamp", "text", ""),
    ("actor", "text", ""),
    ("action", "text", ""),
    ("resource", "text", ""),
    ("chain", "bytea", ""),
    ("timestamp_seconds", "bigint", "s"),
    ("timestamp_nanos", "integer", "s"),
];

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_class c
        JOIN pg_class u ON u.oid='users'::regclass WHERE c.oid=to_regclass('audit_events')
        AND c.relowner=u.relowner AND c.relnamespace=u.relnamespace
        AND c.relkind='r' AND c.relpersistence='p' AND NOT c.relispartition
        AND NOT c.relrowsecurity AND NOT c.relforcerowsecurity
        AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=c.oid OR inhparent=c.oid)
        AND NOT EXISTS(SELECT 1 FROM pg_rewrite WHERE ev_class=c.oid))",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    crate::password_reset_schema::validate_audit_triggers(client)?;
    functions::validate(client)?;
    let schema: String = client
        .query_one(
            "SELECT quote_ident(n.nspname) FROM pg_class c
        JOIN pg_namespace n ON n.oid=c.relnamespace WHERE c.oid='audit_events'::regclass",
            &[],
        )
        .map_err(port)?
        .get(0);
    let rows = client
        .query(
            "SELECT attname::text,format_type(atttypid,atttypmod),attnotnull,
        attgenerated::text,attidentity::text,
        CASE WHEN atttypid='text'::regtype THEN attcollation='pg_catalog.\"default\"'::regcollation
        ELSE attcollation=0 END,
        (SELECT pg_get_expr(adbin,adrelid) FROM pg_attrdef WHERE adrelid=attrelid AND adnum=attnum)
        FROM pg_attribute WHERE attrelid='audit_events'::regclass AND attnum>0 AND NOT attisdropped
        ORDER BY attnum",
            &[],
        )
        .map_err(port)?;
    if rows.len() != COLUMNS.len() {
        return Err(incomplete());
    }
    for (row, (name, kind, generated)) in rows.iter().zip(COLUMNS) {
        let expression: Option<String> = row.get(6);
        let expected = match *name {
            "timestamp_seconds" => Some("audit_timestamp_partstimestamp[1]"),
            "timestamp_nanos" => Some("audit_timestamp_partstimestamp[2]::integer"),
            _ => None,
        };
        if row.get::<_, String>(0) != *name
            || row.get::<_, String>(1) != *kind
            || !row.get::<_, bool>(2)
            || row.get::<_, String>(3) != *generated
            || !row.get::<_, String>(4).is_empty()
            || !row.get::<_, bool>(5)
            || expression
                .as_deref()
                .map(|s| normalize(&s.replace(&format!("{schema}."), "")))
                .as_deref()
                != expected
        {
            return Err(incomplete());
        }
    }
    for (name, primary, unique, columns) in [
        ("audit_events_pkey", true, true, &["sequence"][..]),
        (
            "audit_events_chronological",
            false,
            false,
            &["timestamp_seconds", "timestamp_nanos", "sequence"][..],
        ),
    ] {
        index(client, name, primary, unique, columns)?;
    }
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_index WHERE indrelid='audit_events'::regclass",
            &[],
        )
        .map_err(port)?
        .get(0);
    if count != 2 {
        return Err(incomplete());
    }
    let rows = client
        .query(
            "SELECT conname::text,contype::text,convalidated,condeferrable,condeferred,
        conislocal,coninhcount::integer,conparentid::oid,pg_get_expr(conbin,conrelid),
        (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false',
        ARRAY(SELECT a.attname::text FROM unnest(conkey) WITH ORDINALITY k(num,n)
        JOIN pg_attribute a ON a.attrelid=conrelid AND a.attnum=k.num ORDER BY k.n)
        FROM pg_constraint c WHERE conrelid='audit_events'::regclass AND contype<>'n'",
            &[],
        )
        .map_err(port)?;
    if rows.len() != 3 {
        return Err(incomplete());
    }
    for row in rows {
        let name: String = row.get(0);
        let expression: Option<String> = row.get(8);
        let (kind, expected, columns): (&str, Option<&str>, &[&str]) = match name.as_str() {
            "audit_events_pkey" => ("p", None, &["sequence"]),
            "audit_events_sequence_check" => ("c", Some("sequence>=0"), &["sequence"]),
            "audit_events_chain_check" => ("c", Some("octet_lengthchain=32"), &["chain"]),
            _ => return Err(incomplete()),
        };
        if row.get::<_, String>(1) != kind
            || !row.get::<_, bool>(2)
            || row.get::<_, bool>(3)
            || row.get::<_, bool>(4)
            || !row.get::<_, bool>(5)
            || row.get::<_, i32>(6) != 0
            || row.get::<_, u32>(7) != 0
            || !row.get::<_, bool>(9)
            || expression.as_deref().map(normalize).as_deref() != expected
            || row.get::<_, Vec<String>>(10) != columns
        {
            return Err(incomplete());
        }
    }
    Ok(())
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && !matches!(c, '(' | ')' | '"'))
        .collect()
}

fn index<C: GenericClient>(
    client: &mut C,
    name: &str,
    primary: bool,
    unique: bool,
    columns: &[&str],
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_index i
        JOIN pg_class t ON t.oid=i.indrelid JOIN pg_class c ON c.oid=i.indexrelid
        JOIN pg_am am ON am.oid=c.relam WHERE i.indrelid='audit_events'::regclass
        AND c.relname=$1 AND c.relnamespace=t.relnamespace AND c.relowner=t.relowner
        AND c.relkind='i' AND c.relpersistence='p' AND NOT c.relispartition AND am.amname='btree'
        AND i.indisprimary=$2 AND i.indisunique=$3 AND i.indisvalid AND i.indisready
        AND i.indislive AND i.indimmediate AND NOT i.indisexclusion AND NOT i.indnullsnotdistinct
        AND i.indexprs IS NULL AND i.indpred IS NULL
        AND i.indnatts=cardinality($4::text[]) AND i.indnkeyatts=cardinality($4::text[])
        AND ARRAY(SELECT a.attname::text FROM unnest(i.indkey::smallint[]) WITH ORDINALITY k(num,n)
            JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.num ORDER BY k.n)=$4::text[]
        AND NOT EXISTS(SELECT 1 FROM unnest(i.indkey::smallint[],i.indclass::oid[],i.indcollation::oid[],i.indoption::smallint[]) k(num,op,coll,options)
            JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.num
            LEFT JOIN pg_opclass op ON op.oid=k.op WHERE op.opcnamespace<>'pg_catalog'::regnamespace
            OR op.opcmethod<>c.relam OR NOT op.opcdefault OR op.opcintype<>a.atttypid
            OR k.coll<>a.attcollation OR k.options<>0)
        AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=i.indexrelid OR inhparent=i.indexrelid))",
        &[&name, &primary, &unique, &columns]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
