use super::{constraints, functions, incomplete, port, FUNCTIONS, SEQUENCE, SOURCES, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_class WHERE oid=to_regclass($1)
        AND relkind='r' AND relpersistence='p' AND NOT relispartition AND NOT relrowsecurity
        AND NOT relforcerowsecurity AND NOT EXISTS(SELECT 1 FROM pg_inherits WHERE
        inhrelid=to_regclass($1) OR inhparent=to_regclass($1)))",
            &[&TABLE],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    columns(client)?;
    constraints::validate(client)?;
    functions::validate(client)?;
    sequence(client)?;
    for table in SOURCES {
        trigger(client, table, "deadline_source_emit", FUNCTIONS[2], 5)?;
    }
    trigger(client, TABLE, "deadline_source_insert", FUNCTIONS[1], 7)?;
    trigger(client, TABLE, "deadline_source_immutable", FUNCTIONS[0], 58)?;
    let invalid: bool = client.query_one("SELECT
        EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='deadline_source_events'::regclass AND tgenabled NOT IN ('O','A'))
        OR (SELECT count(*) FROM pg_trigger WHERE tgrelid='deadline_source_events'::regclass AND NOT tgisinternal)<>2",
        &[]).map_err(port)?.get(0);
    if invalid {
        return Err(incomplete());
    }
    Ok(())
}

fn columns<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let expected = [
        ("sequence", "bigint", true, ""),
        ("source_kind", "text", true, ""),
        ("source_id", "uuid", true, ""),
        ("revision", "bigint", true, ""),
        ("case_id", "uuid", false, ""),
        ("hearing_id", "uuid", false, ""),
        ("operation_id", "uuid", true, ""),
        ("fact_family", "text", false, "s"),
        ("hearing_result_id", "uuid", false, "s"),
        ("calendar_id", "uuid", false, "s"),
        ("profile_id", "uuid", false, "s"),
    ];
    let rows = client
        .query(
            "SELECT attname::text,format_type(atttypid,atttypmod),attnotnull,
        attgenerated::text,attidentity::text FROM pg_attribute
        WHERE attrelid='deadline_source_events'::regclass AND attnum>0 AND NOT attisdropped",
            &[],
        )
        .map_err(port)?;
    if rows.len() != expected.len()
        || expected.iter().any(|(name, kind, required, generated)| {
            !rows.iter().any(|r| {
                r.get::<_, String>(0) == *name
                    && r.get::<_, String>(1) == *kind
                    && r.get::<_, bool>(2) == *required
                    && r.get::<_, String>(3) == *generated
                    && r.get::<_, String>(4).is_empty()
            })
        })
    {
        return Err(incomplete());
    }
    Ok(())
}

fn sequence<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_class c
        JOIN pg_sequence s ON s.seqrelid=c.oid JOIN pg_class t ON t.oid='deadline_source_events'::regclass
        WHERE c.oid=to_regclass($1) AND c.relnamespace=t.relnamespace AND c.relkind='S'
        AND c.relpersistence='p' AND s.seqtypid='bigint'::regtype AND s.seqstart=1 AND s.seqincrement=1
        AND s.seqmin=1 AND s.seqmax=9223372036854775807 AND s.seqcache=1 AND NOT s.seqcycle
        AND pg_get_serial_sequence(t.oid::regclass::text,'sequence')::regclass=c.oid
        AND EXISTS(SELECT 1 FROM pg_depend d JOIN pg_attribute a ON a.attrelid=t.oid AND a.attname='sequence'
            WHERE d.classid='pg_class'::regclass AND d.objid=c.oid AND d.objsubid=0
            AND d.refclassid='pg_class'::regclass AND d.refobjid=t.oid AND d.refobjsubid=a.attnum AND d.deptype='a'))",
        &[&SEQUENCE]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}

fn trigger<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    function: &str,
    kind: i16,
) -> Result<(), ApplicationError> {
    let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_trigger
        WHERE tgrelid=$1::text::regclass AND tgname=$2 AND tgfoid=$3::text::regprocedure AND tgtype=$4
        AND tgenabled IN ('O','A') AND NOT tgisinternal AND NOT tgdeferrable AND NOT tginitdeferred
        AND tgqual IS NULL AND tgnargs=0 AND tgattr=''::int2vector)", &[&table,&name,&function,&kind]).map_err(port)?.get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
