use super::{checks, incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

const RECEIPTS: &str = "SELECT EXISTS(
    SELECT 1 FROM individual_document_signatures r
    LEFT JOIN document_signing_bindings b ON b.binding_id=r.binding_id
    LEFT JOIN users u ON u.id=r.signer_id
    LEFT JOIN documents d ON d.id=r.document_id AND d.version=r.document_version
    LEFT JOIN document_series s ON s.id=r.document_id AND s.case_id=r.case_id
    LEFT JOIN participant_credential_trust_revisions t
        ON t.deployment_id=r.deployment_id AND t.revision=r.trust_revision
    LEFT JOIN participant_credential_authority authority ON authority.deployment_id=r.deployment_id
    LEFT JOIN audit_events a ON a.sequence=r.audit_sequence
    LEFT JOIN document_signing_withdrawals w ON w.binding_id=r.binding_id
    WHERE b.binding_id IS NULL OR u.id IS NULL OR d.id IS NULL OR s.id IS NULL
        OR t.revision IS NULL OR authority.deployment_id IS NULL OR a.sequence IS NULL
        OR b.signer_id IS DISTINCT FROM r.signer_id OR b.leaf_fingerprint IS DISTINCT FROM r.leaf_fingerprint
        OR b.deployment_id IS DISTINCT FROM r.deployment_id OR b.root_fingerprint IS DISTINCT FROM r.root_fingerprint
        OR b.account_revision>r.account_revision OR b.auth_generation>r.auth_generation
        OR b.trust_revision>r.trust_revision OR b.audit_sequence>=r.audit_sequence
        OR u.revision<r.account_revision OR u.auth_generation<r.auth_generation
        OR d.case_id IS DISTINCT FROM r.case_id OR d.digest IS DISTINCT FROM r.content_digest
        OR r.root_fingerprint IS DISTINCT FROM authority.root_fingerprint
        OR r.valid_from IS DISTINCT FROM greatest(b.certificate_not_before,t.valid_from)
        OR r.valid_until IS DISTINCT FROM least(b.certificate_not_after,t.valid_until)
        OR ROW(r.accepted_at_seconds,r.accepted_at_nanoseconds)
            <ROW(t.published_at_seconds,t.published_at_nanoseconds)
        OR ROW(r.accepted_at_seconds,r.accepted_at_nanoseconds)
            <ROW(b.registered_at_seconds,b.registered_at_nanoseconds)
        OR (w.binding_id IS NOT NULL AND (w.audit_sequence<=r.audit_sequence
            OR w.account_revision<r.account_revision OR w.auth_generation<r.auth_generation
            OR ROW(w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds)
                <ROW(r.accepted_at_seconds,r.accepted_at_nanoseconds)))
        OR r.consent IS DISTINCT FROM (convert_to('DOCSIGN1','UTF8') || decode('0101','hex')
            || uuid_send(r.deployment_id) || r.root_fingerprint
            || substring(int8send(r.trust_revision) FROM 5 FOR 4)
            || uuid_send(r.signer_id) || int8send(r.account_revision) || int8send(r.auth_generation)
            || uuid_send(r.binding_id) || r.leaf_fingerprint || uuid_send(r.operation_id)
            || uuid_send(r.case_id) || uuid_send(r.document_id)
            || substring(int8send(r.document_version) FROM 5 FOR 4)
            || r.content_digest || sha256(r.pdf_signature))
        OR a.actor IS DISTINCT FROM r.actor_email OR a.action IS DISTINCT FROM 'document.individually_signed'
        OR a.resource IS DISTINCT FROM ('case:' || r.case_id::text || ':document:' || r.document_id::text
            || ':version:' || r.document_version::text || ':signer:' || r.signer_id::text
            || ':operation:' || r.operation_id::text || ':consent:' || encode(r.consent_digest,'hex'))
        OR a.timestamp_seconds IS DISTINCT FROM r.accepted_at_seconds
        OR a.timestamp_nanos IS DISTINCT FROM r.accepted_at_nanoseconds)";

const PAIRS: &str = "SELECT EXISTS(SELECT 1 FROM documents d
    LEFT JOIN individual_document_signatures r ON r.document_id=d.id AND r.document_version=d.version
    WHERE (d.evidence->'individual' IS NOT NULL AND d.evidence->'individual'<>'null'::jsonb)
        IS DISTINCT FROM (r.operation_id IS NOT NULL))
    OR EXISTS(SELECT 1 FROM audit_events a WHERE a.action='document.individually_signed'
        AND NOT EXISTS(SELECT 1 FROM individual_document_signatures r WHERE r.audit_sequence=a.sequence))";

/// Checks historical links; the store then validates captured public proofs and exact evidence.
pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let predicates = checks::expected(TABLES[0]).join(" AND ");
    let shape = format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE ({predicates}) IS NOT TRUE)",
        TABLES[0]
    );
    for query in [&shape, RECEIPTS, PAIRS] {
        if client
            .query_one(query, &[])
            .map_err(port)?
            .get::<_, bool>(0)
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
