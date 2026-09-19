//! Exact CHECK predicates, including null pairing and finite UTC ranges.
use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn expected(table: &str) -> &'static [&'static str] {
    match table {
        "alert_preferences" => &[
            "((revision >= 1) AND (revision <= '4294967295'::bigint))",
            "alert_time_valid(recorded_seconds, recorded_nanos)",
            "alert_payload_valid(payload, payload_digest)",
        ],
        "alert_subject_state" => &[
            "(kind = ANY (ARRAY[0, 1]))", "(generation > 0)",
            "alert_payload_valid(payload, payload_digest)",
        ],
        "alert_scan_cursor" => &[
            "singleton", "(kind = ANY (ARRAY[0, 1]))", "(cycle >= 0)",
            "alert_optional_time_valid(next_seconds, next_nanos)",
            "(((active_kind IS NULL) AND (active_id IS NULL) AND (after_recipient IS NULL)) OR ((active_kind = ANY (ARRAY[0, 1])) AND (active_id IS NOT NULL)))",
        ],
        "alert_schedule" => &[
            "((length(occurrence_key) >= 1) AND (length(occurrence_key) <= 200))",
            "(status = ANY (ARRAY['planned'::text, 'activated'::text, 'superseded'::text]))",
            "(generation > 0)", "alert_time_valid(trigger_seconds, trigger_nanos)",
            "alert_payload_valid(payload, payload_digest)",
        ],
        "alert_notifications" => &[
            "alert_time_valid(created_seconds, created_nanos)",
            "alert_optional_time_valid(read_seconds, read_nanos)",
            "alert_optional_time_valid(resolved_seconds, resolved_nanos)",
            "((resolved_seconds IS NULL) = (resolved_reason IS NULL))",
            "((resolved_reason IS NULL) OR (resolved_reason = ANY (ARRAY['superseded'::text, 'attention_recorded'::text, 'target_retired'::text, 'cancelled_hearing'::text, 'no_longer_eligible'::text])))",
            "alert_payload_valid(payload, payload_digest)",
        ],
        "alert_read_receipts" => &["alert_time_valid(read_seconds, read_nanos)"],
        "alert_email_outbox" => &[
            "(status = ANY (ARRAY['pending'::text, 'sending'::text, 'accepted'::text, 'failed'::text, 'unknown'::text, 'cancelled'::text, 'disabled'::text]))",
            "(sequence >= 0)", "alert_optional_time_valid(next_seconds, next_nanos)",
            "alert_payload_valid(payload, payload_digest)",
        ],
        "alert_email_attempts" => &[
            "(sequence >= 0)", "alert_payload_valid(payload, payload_digest)",
        ],
        _ => &[],
    }
}

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client
            .query(
                "SELECT pg_get_expr(c.conbin,c.conrelid),c.convalidated AND NOT c.connoinherit
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.contype='c'",
                &[&table],
            )
            .map_err(port)?;
        let mut actual: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
        let mut expected: Vec<String> = expected(table)
            .iter()
            .map(|value| (*value).into())
            .collect();
        actual.sort();
        expected.sort();
        if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(1)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
