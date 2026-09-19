use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [(&'static str, &'static str, bool)] {
    match table {
        "case_procedural_resources" => &[
            ("id", "uuid", true),
            ("case_id", "uuid", true),
            ("initial_revision", "bigint", true),
        ],
        "case_procedural_resource_acts" => &[
            ("id", "uuid", true),
            ("resource_id", "uuid", true),
            ("case_id", "uuid", true),
            ("initial_resource_revision", "bigint", true),
        ],
        "case_procedural_resource_revisions" => &[
            ("resource_id", "uuid", true),
            ("case_id", "uuid", true),
            ("revision", "bigint", true),
            ("operation_id", "uuid", true),
            ("action", "text", true),
            ("status", "text", true),
            ("kind", "text", true),
            ("reason", "text", false),
            ("values_canonical", "bytea", true),
            ("values_view", "jsonb", true),
            ("values_digest", "bytea", true),
            ("sources_canonical", "bytea", true),
            ("sources_digest", "bytea", true),
            ("supports_view", "jsonb", true),
            ("submission_canonical", "bytea", true),
            ("submission_digest", "bytea", true),
            ("capture_canonical", "bytea", true),
            ("capture_digest", "bytea", true),
            ("previous_capture_digest", "bytea", false),
            ("act_id", "uuid", false),
            ("act_revision", "bigint", false),
            ("act_values_canonical", "bytea", false),
            ("act_values_view", "jsonb", false),
            ("act_supports_view", "jsonb", false),
            ("act_previous_resource_revision", "bigint", false),
            ("act_previous_capture_digest", "bytea", false),
            ("recorded_administration_revision", "bigint", false),
            ("recorded_administration_digest", "bytea", false),
            ("recorded_administration_title", "text", false),
            ("recorded_administration_reference", "text", false),
            ("recorded_stage_revision", "bigint", false),
            ("recorded_at_seconds", "bigint", true),
            ("recorded_at_nanoseconds", "integer", true),
            ("recorded_by", "uuid", true),
            ("recorded_by_email", "text", true),
        ],
        _ => &[],
    }
}
pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|c| c.0).collect()
}

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let expected = expected(table);
        let rows = client
            .query(
                "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"default\"'::regcollation
                    ELSE a.attcollation=0 END,
                (SELECT pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum",
                &[&table],
            )
            .map_err(port)?;
        if rows.len() != expected.len()
            || rows
                .iter()
                .zip(expected)
                .any(|(row, (name, kind, required))| {
                    row.get::<_, String>(0) != *name
                        || row.get::<_, String>(1) != *kind
                        || row.get::<_, bool>(2) != *required
                        || !row.get::<_, String>(3).is_empty()
                        || !row.get::<_, String>(4).is_empty()
                        || !row.get::<_, bool>(5)
                        || row.get::<_, Option<String>>(6).as_deref()
                            != if table == "case_procedural_resources"
                                && *name == "initial_revision"
                            {
                                Some("1")
                            } else {
                                None
                            }
                })
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
