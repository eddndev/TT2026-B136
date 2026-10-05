use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [(&'static str, &'static str, bool)] {
    match table {
        "case_measure_operations" => &[
            ("operation_id", "uuid", true),
            ("case_id", "uuid", true),
            ("family", "text", true),
            ("owner_digest", "bytea", true),
            ("audit_sequence", "bigint", true),
        ],
        "case_measure_decisions" => &[
            ("decision_id", "uuid", true),
            ("operation_id", "uuid", true),
            ("case_id", "uuid", true),
            ("values_canonical", "bytea", true),
            ("values_view", "jsonb", true),
            ("values_digest", "bytea", true),
            ("outcome_canonical", "bytea", true),
            ("outcome_view", "jsonb", true),
            ("outcome_digest", "bytea", true),
            ("observed_administration_revision", "bigint", true),
            ("observed_stage_revision", "bigint", true),
            ("observed_context_digest", "bytea", true),
            ("support_format", "text", true),
            ("support_policy", "text", true),
            ("recorded_by", "uuid", true),
            ("recorded_by_email", "text", true),
            ("recorded_by_role", "text", true),
            ("recorded_at_seconds", "bigint", true),
            ("recorded_at_nanoseconds", "integer", true),
            ("submission_digest", "bytea", true),
            ("review_digest", "bytea", true),
            ("decision_digest", "bytea", true),
            ("group_digest", "bytea", true),
            ("anchor_kind", "text", true),
            ("anchor_hearing_id", "uuid", false),
            ("anchor_revision", "bigint", false),
            ("anchor_values_digest", "bytea", false),
            ("anchor_submission_digest", "bytea", false),
            ("anchor_precautionary_hearing_id", "uuid", false),
            ("anchor_capture_digest", "bytea", false),
        ],
        "case_measures" => &[
            ("id", "uuid", true),
            ("case_id", "uuid", true),
            ("initial_revision", "bigint", true),
            ("root_operation", "uuid", true),
        ],
        "case_measure_revisions" => &[
            ("measure_id", "uuid", true),
            ("revision", "bigint", true),
            ("case_id", "uuid", true),
            ("owner_operation", "uuid", true),
            ("family", "text", true),
            ("action", "text", true),
            ("values_canonical", "bytea", true),
            ("values_view", "jsonb", true),
            ("values_digest", "bytea", true),
            ("capture_digest", "bytea", true),
            ("subject_id", "uuid", true),
            ("subject_revision", "bigint", true),
            ("subject_values_digest", "bytea", true),
            ("supervisor_id", "uuid", false),
            ("supervisor_revision", "bigint", false),
        ],
        _ => &[],
    }
}

pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|column| column.0).collect()
}

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let expected = expected(table);
        let rows = client.query(
            "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"default\"'::regcollation ELSE a.attcollation=0 END,
                (SELECT pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum", &[&table],
        ).map_err(port)?;
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
                            != if table == "case_measures" && *name == "initial_revision" {
                                Some("1")
                            } else if table == "case_measure_decisions" && *name == "anchor_kind" {
                                Some("'none'::text")
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
