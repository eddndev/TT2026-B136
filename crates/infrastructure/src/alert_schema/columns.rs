use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [(&'static str, &'static str, bool)] {
    match table {
        "alert_preferences" => &[
            ("user_id", "uuid", true),
            ("revision", "bigint", true),
            ("operation_id", "uuid", true),
            ("recorded_seconds", "bigint", true),
            ("recorded_nanos", "integer", true),
            ("payload", "bytea", true),
            ("payload_digest", "bytea", true),
        ],
        "alert_subject_state" => &[
            ("kind", "smallint", true),
            ("id", "uuid", true),
            ("case_id", "uuid", true),
            ("generation", "bigint", true),
            ("dirty", "boolean", true),
            ("payload", "bytea", true),
            ("payload_digest", "bytea", true),
        ],
        "alert_scan_cursor" => &[
            ("singleton", "boolean", true),
            ("kind", "smallint", true),
            ("id", "uuid", false),
            ("active_kind", "smallint", false),
            ("active_id", "uuid", false),
            ("after_recipient", "uuid", false),
            ("cycle", "bigint", true),
            ("next_seconds", "bigint", false),
            ("next_nanos", "integer", false),
        ],
        "alert_schedule" => &[
            ("id", "uuid", true),
            ("kind", "smallint", true),
            ("subject_id", "uuid", true),
            ("case_id", "uuid", true),
            ("recipient", "uuid", true),
            ("occurrence_key", "text", true),
            ("occurrence_id", "uuid", true),
            ("trigger_seconds", "bigint", true),
            ("trigger_nanos", "integer", true),
            ("status", "text", true),
            ("generation", "bigint", true),
            ("payload", "bytea", true),
            ("payload_digest", "bytea", true),
        ],
        "alert_notifications" => &[
            ("id", "uuid", true),
            ("schedule_id", "uuid", true),
            ("recipient", "uuid", true),
            ("kind", "smallint", true),
            ("subject_id", "uuid", true),
            ("case_id", "uuid", true),
            ("created_seconds", "bigint", true),
            ("created_nanos", "integer", true),
            ("internal_enabled", "boolean", true),
            ("payload", "bytea", true),
            ("payload_digest", "bytea", true),
            ("read_seconds", "bigint", false),
            ("read_nanos", "integer", false),
            ("resolved_seconds", "bigint", false),
            ("resolved_nanos", "integer", false),
            ("resolved_reason", "text", false),
        ],
        "alert_read_receipts" => &[
            ("operation_id", "uuid", true),
            ("recipient", "uuid", true),
            ("alert_id", "uuid", true),
            ("read_seconds", "bigint", true),
            ("read_nanos", "integer", true),
        ],
        "alert_email_outbox" => &[
            ("id", "uuid", true),
            ("alert_id", "uuid", true),
            ("status", "text", true),
            ("next_seconds", "bigint", false),
            ("next_nanos", "integer", false),
            ("sequence", "bigint", true),
            ("payload", "bytea", true),
            ("payload_digest", "bytea", true),
        ],
        "alert_email_attempts" => &[
            ("delivery_id", "uuid", true),
            ("sequence", "bigint", true),
            ("payload", "bytea", true),
            ("payload_digest", "bytea", true),
        ],
        _ => &[],
    }
}

pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|column| column.0).collect()
}

pub(super) fn mutable(table: &str) -> &'static [&'static str] {
    match table {
        "alert_subject_state" => &["generation", "dirty", "payload", "payload_digest"],
        "alert_scan_cursor" => &[
            "kind",
            "id",
            "active_kind",
            "active_id",
            "after_recipient",
            "cycle",
            "next_seconds",
            "next_nanos",
        ],
        "alert_schedule" => &["generation", "status", "payload", "payload_digest"],
        "alert_notifications" => &[
            "read_seconds",
            "read_nanos",
            "resolved_seconds",
            "resolved_nanos",
            "resolved_reason",
        ],
        "alert_email_outbox" => &[
            "status",
            "next_seconds",
            "next_nanos",
            "sequence",
            "payload",
            "payload_digest",
        ],
        _ => &[],
    }
}

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let expected = expected(table);
        let rows = client
            .query(
                "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"C\"'::regcollation
                    ELSE a.attcollation=0 END,
                EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
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
                        || row.get::<_, bool>(6)
                })
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
