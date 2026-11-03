use super::{incomplete, port, TABLES};
use crate::postgres::CatalogClient;
use application::ApplicationError;

/// Each CHECK constraint as `pg_get_expr` prints it.
pub(super) fn expected(table: &str) -> Vec<String> {
    let values: &[&str] = match table {
        "contracts" => &[
            "((char_length(title) >= 1) AND (char_length(title) <= 160))",
            "((created_at_nanoseconds >= 0) AND (created_at_nanoseconds <= 999999999))",
            "((created_at_seconds >= 0) AND (created_at_seconds <= '253402300799'::bigint))",
            "((octet_length(created_by_email) >= 1) AND (octet_length(created_by_email) <= 1280))",
            "(case_id <> '00000000-0000-0000-0000-000000000000'::uuid)",
            "(document_id <> '00000000-0000-0000-0000-000000000000'::uuid)",
            "(id <> '00000000-0000-0000-0000-000000000000'::uuid)",
        ],
        "contract_signature_requests" => &[
            "(((expires_at_seconds - created_at_seconds) >= 86400) AND ((expires_at_seconds - created_at_seconds) <= 2592000))",
            "((char_length(ceremony_reference) >= 1) AND (char_length(ceremony_reference) <= 80))",
            "((client_id <> firm_signer_id) AND (client_id <> requester_id))",
            "((client_subject_id IS NULL) = (client_subject_revision IS NULL))",
            "((created_at_nanoseconds >= 0) AND (created_at_nanoseconds <= 999999999))",
            "((created_at_seconds >= 0) AND (created_at_seconds <= '253402300799'::bigint))",
            "((document_version >= 1) AND (document_version <= '4294967295'::bigint))",
            "((octet_length(requester_email) >= 1) AND (octet_length(requester_email) <= 1280))",
            "(client_leaf <> firm_leaf)",
            "(deployment_id <> '00000000-0000-0000-0000-000000000000'::uuid)",
            "(expires_at_seconds <= '253402300799'::bigint)",
            "(octet_length(content_digest) = 32)",
            "(octet_length(statement) = 240)",
            "(request_id <> '00000000-0000-0000-0000-000000000000'::uuid)",
            "(statement_digest = sha256(statement))",
        ],
        "contract_events" => &[
            "((actor_id IS NULL) = (kind = 'expired'::text))",
            "((at_nanoseconds >= 0) AND (at_nanoseconds <= 999999999))",
            "((at_seconds >= 0) AND (at_seconds <= '253402300799'::bigint))",
            "((kind <> 'expired'::text) OR (actor_label = 'system:contract-expiry'::text))",
            "((kind <> ALL (ARRAY['abandoned'::text, 'cancelled'::text])) OR (reason IS NOT NULL))",
            "((kind = ANY (ARRAY['abandoned'::text, 'cancelled'::text, 'rejected'::text, 'declined'::text])) OR (reason IS NULL))",
            "((octet_length(actor_label) >= 1) AND (octet_length(actor_label) <= 1280))",
            "((reason IS NULL) OR ((char_length(reason) >= 1) AND (char_length(reason) <= 500)))",
            "((request_id IS NULL) = (kind = ANY (ARRAY['created'::text, 'abandoned'::text])))",
            "((revision = 1) = (kind = 'created'::text))",
            "((revision >= 1) AND (revision <= '4294967295'::bigint))",
            "(audit_sequence >= 0)",
            "(kind = ANY (ARRAY['created'::text, 'abandoned'::text, 'requested'::text, 'client_signed'::text, 'rejected'::text, 'completed'::text, 'declined'::text, 'cancelled'::text, 'expired'::text]))",
            "(operation_id <> '00000000-0000-0000-0000-000000000000'::uuid)",
        ],
        "contract_party_signatures" => &[
            "(\"position\" = ANY (ARRAY[1, 2]))",
            "((\"position\" = 1) = (timestamp_token IS NOT NULL))",
            "((accepted_at_nanoseconds >= 0) AND (accepted_at_nanoseconds <= 999999999))",
            "((accepted_at_seconds >= 0) AND (accepted_at_seconds <= '253402300799'::bigint))",
            "((accepted_at_seconds >= checked_at) AND (accepted_at_seconds <= valid_until))",
            "((account_revision >= 0) AND (auth_generation >= 0) AND (auth_generation <= account_revision))",
            "((octet_length(actor_email) >= 1) AND (octet_length(actor_email) <= 1280))",
            "((octet_length(timestamp_token) >= 1) AND (octet_length(timestamp_token) <= 65536))",
            "((octet_length(tsa_chain_pem) >= 1) AND (octet_length(tsa_chain_pem) <= 65536))",
            "((timestamp_token IS NULL) = (tsa_chain_pem IS NULL))",
            "((trust_revision >= 1) AND (trust_revision <= '4294967295'::bigint))",
            "((valid_from <= checked_at) AND (checked_at <= valid_until))",
            "(octet_length(leaf_fingerprint) = 32)",
            "(octet_length(root_fingerprint) = 32)",
            "(octet_length(signature) = 384)",
            "(octet_length(statement) = 303)",
            "(statement_digest = sha256(statement))",
        ],
        "contract_completions" => &[
            "((completed_at_nanoseconds >= 0) AND (completed_at_nanoseconds <= 999999999))",
            "((completed_at_seconds >= 0) AND (completed_at_seconds <= '253402300799'::bigint))",
            "((octet_length(document_token) >= 1) AND (octet_length(document_token) <= 65536))",
            "((octet_length(manifest) >= 1) AND (octet_length(manifest) <= 65536))",
            "((octet_length(manifest_token) >= 1) AND (octet_length(manifest_token) <= 65536))",
            "((octet_length(tsa_chain_pem) >= 1) AND (octet_length(tsa_chain_pem) <= 65536))",
            "(manifest_sha256 = sha256(manifest))",
        ],
        "contract_notifications" => &[
            "((char_length(message_text) >= 1) AND (char_length(message_text) <= 4096))",
            "((char_length(subject) >= 1) AND (char_length(subject) <= 200))",
            "((created_at_seconds >= 0) AND (created_at_seconds <= '253402300799'::bigint))",
            "((lease_id IS NULL) = (lease_until IS NULL))",
            "((octet_length(message_from) >= 3) AND (octet_length(message_from) <= 320))",
            "((octet_length(message_to) >= 3) AND (octet_length(message_to) <= 320))",
            "(delivery_generation >= 0)",
            "(delivery_state = ANY (ARRAY['pending'::text, 'sent'::text, 'uncertain'::text, 'rejected'::text]))",
            "(id <> '00000000-0000-0000-0000-000000000000'::uuid)",
            "(kind = ANY (ARRAY['requested'::text, 'client_signed'::text, 'rejected'::text, 'completed'::text, 'declined'::text, 'cancelled'::text, 'expired'::text]))",
        ],
        _ => &[],
    };
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn validate<C: CatalogClient>(client: &mut C) -> Result<(), ApplicationError> {
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
        let mut expected = expected(table);
        actual.sort();
        expected.sort();
        if actual != expected || rows.iter().any(|row| !row.get::<_, bool>(1)) {
            return Err(incomplete());
        }
    }
    Ok(())
}
