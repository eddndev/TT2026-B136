use super::{incomplete, port, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

/// Name, type and whether the column admits NULL.
type Column = (&'static str, &'static str, bool);

fn expected(table: &str) -> &'static [Column] {
    match table {
        "contracts" => &[
            ("id", "uuid", false),
            ("case_id", "uuid", false),
            ("document_id", "uuid", false),
            ("title", "text", false),
            ("created_by", "uuid", false),
            ("created_by_email", "text", false),
            ("created_at_seconds", "bigint", false),
            ("created_at_nanoseconds", "integer", false),
        ],
        "contract_signature_requests" => &[
            ("request_id", "uuid", false),
            ("contract_id", "uuid", false),
            ("case_id", "uuid", false),
            ("document_id", "uuid", false),
            ("document_version", "bigint", false),
            ("content_digest", "bytea", false),
            ("client_id", "uuid", false),
            ("firm_signer_id", "uuid", false),
            ("requester_id", "uuid", false),
            ("requester_email", "text", false),
            ("client_binding_id", "uuid", false),
            ("client_leaf", "bytea", false),
            ("firm_binding_id", "uuid", false),
            ("firm_leaf", "bytea", false),
            ("client_subject_id", "uuid", true),
            ("client_subject_revision", "bigint", true),
            ("ceremony_reference", "text", false),
            ("deployment_id", "uuid", false),
            ("statement", "bytea", false),
            ("statement_digest", "bytea", false),
            ("created_at_seconds", "bigint", false),
            ("created_at_nanoseconds", "integer", false),
            ("expires_at_seconds", "bigint", false),
        ],
        "contract_events" => &[
            ("contract_id", "uuid", false),
            ("revision", "bigint", false),
            ("kind", "text", false),
            ("request_id", "uuid", true),
            ("actor_id", "uuid", true),
            ("actor_label", "text", false),
            ("reason", "text", true),
            ("at_seconds", "bigint", false),
            ("at_nanoseconds", "integer", false),
            ("operation_id", "uuid", false),
            ("audit_sequence", "bigint", false),
        ],
        "contract_party_signatures" => &[
            ("request_id", "uuid", false),
            ("position", "smallint", false),
            ("operation_id", "uuid", false),
            ("signer_id", "uuid", false),
            ("binding_id", "uuid", false),
            ("leaf_fingerprint", "bytea", false),
            ("deployment_id", "uuid", false),
            ("trust_revision", "bigint", false),
            ("root_fingerprint", "bytea", false),
            ("account_revision", "bigint", false),
            ("auth_generation", "bigint", false),
            ("actor_email", "text", false),
            ("statement", "bytea", false),
            ("statement_digest", "bytea", false),
            ("signature", "bytea", false),
            ("checked_at", "bigint", false),
            ("valid_from", "bigint", false),
            ("valid_until", "bigint", false),
            ("accepted_at_seconds", "bigint", false),
            ("accepted_at_nanoseconds", "integer", false),
            ("timestamp_token", "bytea", true),
            ("tsa_chain_pem", "bytea", true),
        ],
        "contract_completions" => &[
            ("request_id", "uuid", false),
            ("document_token", "bytea", false),
            ("manifest", "bytea", false),
            ("manifest_sha256", "bytea", false),
            ("manifest_token", "bytea", false),
            ("tsa_chain_pem", "bytea", false),
            ("completed_at_seconds", "bigint", false),
            ("completed_at_nanoseconds", "integer", false),
        ],
        "contract_notifications" => &[
            ("id", "uuid", false),
            ("request_id", "uuid", false),
            ("contract_id", "uuid", false),
            ("event_revision", "bigint", false),
            ("recipient_id", "uuid", false),
            ("kind", "text", false),
            ("message_from", "text", false),
            ("message_to", "text", false),
            ("subject", "text", false),
            ("message_text", "text", false),
            ("created_at_seconds", "bigint", false),
            ("delivery_state", "text", false),
            ("delivery_generation", "bigint", false),
            ("lease_id", "uuid", true),
            ("lease_until", "bigint", true),
            ("provider_deadline", "bigint", true),
            ("retry_at", "bigint", true),
        ],
        _ => &[],
    }
}

pub(super) fn names(table: &str) -> Vec<&'static str> {
    expected(table).iter().map(|column| column.0).collect()
}

/// Exact names, order, types and nullability, without defaults, generated
/// or identity columns, and with the default collation on text.
pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let rows = client.query(
            "SELECT a.attname::text,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                a.attgenerated::text,a.attidentity::text,
                CASE WHEN a.atttypid='pg_catalog.text'::regtype
                    THEN a.attcollation='pg_catalog.\"default\"'::regcollation ELSE a.attcollation=0 END,
                EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum)
            FROM pg_attribute a WHERE a.attrelid=$1::text::regclass
                AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum", &[&table],
        ).map_err(port)?;
        if rows.len() != expected(table).len()
            || rows
                .iter()
                .zip(expected(table))
                .any(|(row, (name, kind, nullable))| {
                    row.get::<_, String>(0) != *name
                        || row.get::<_, String>(1) != *kind
                        || row.get::<_, bool>(2) == *nullable
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
