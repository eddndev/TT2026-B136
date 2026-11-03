//! Stored links of contract history, checked in SQL before any byte is
//! decoded: event chains and their audit rows, the rebuilt statements and the
//! rows each event pairs with.
use super::{checks, inconsistent, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

const ROOTS: &str = "SELECT EXISTS(SELECT 1 FROM contracts c
        LEFT JOIN contract_events e ON e.contract_id=c.id AND e.revision=1
        WHERE e.kind IS DISTINCT FROM 'created' OR e.actor_id IS DISTINCT FROM c.created_by
            OR e.actor_label IS DISTINCT FROM c.created_by_email
            OR ROW(e.at_seconds,e.at_nanoseconds)
                IS DISTINCT FROM ROW(c.created_at_seconds,c.created_at_nanoseconds))
    OR EXISTS(SELECT 1 FROM contract_events GROUP BY contract_id HAVING count(*)<>max(revision))
    OR EXISTS(SELECT 1 FROM contract_events e JOIN contract_events p
            ON p.contract_id=e.contract_id AND p.revision=e.revision-1
        WHERE p.audit_sequence>=e.audit_sequence
            OR ROW(e.at_seconds,e.at_nanoseconds)<ROW(p.at_seconds,p.at_nanoseconds)
            OR NOT ((e.kind IN ('requested','abandoned')
                    AND p.kind IN ('created','rejected','declined','cancelled','expired'))
                OR (p.request_id=e.request_id AND ((p.kind='requested'
                    AND e.kind IN ('client_signed','rejected','cancelled','expired'))
                    OR (p.kind='client_signed'
                        AND e.kind IN ('completed','declined','cancelled','expired'))))))";

const AUDIT: &str = "SELECT EXISTS(SELECT 1 FROM contract_events e
        JOIN contracts c ON c.id=e.contract_id
        LEFT JOIN audit_events a ON a.sequence=e.audit_sequence
        WHERE a.sequence IS NULL OR a.actor IS DISTINCT FROM e.actor_label
            OR a.action IS DISTINCT FROM 'contract.' || e.kind
            OR a.resource IS DISTINCT FROM CASE e.kind
                WHEN 'created' THEN 'contract:' || c.id::text || ':document:' || c.document_id::text
                WHEN 'abandoned' THEN 'contract:' || c.id::text || ':revision:' || e.revision::text
                ELSE 'contract-request:' || e.request_id::text || ':revision:' || e.revision::text END
            OR a.timestamp_seconds IS DISTINCT FROM e.at_seconds
            OR a.timestamp_nanos IS DISTINCT FROM e.at_nanoseconds)
    OR EXISTS(SELECT 1 FROM audit_events a WHERE a.action IN ('contract.created','contract.abandoned',
            'contract.requested','contract.client_signed','contract.rejected','contract.completed',
            'contract.declined','contract.cancelled','contract.expired')
        AND NOT EXISTS(SELECT 1 FROM contract_events e WHERE e.audit_sequence=a.sequence))";

const REQUESTS: &str = "SELECT EXISTS(SELECT 1 FROM contract_signature_requests r
        LEFT JOIN contract_events e ON e.request_id=r.request_id AND e.kind='requested'
        LEFT JOIN documents d ON d.id=r.document_id AND d.version=r.document_version
        LEFT JOIN document_signing_bindings cb ON cb.binding_id=r.client_binding_id
        LEFT JOIN document_signing_bindings fb ON fb.binding_id=r.firm_binding_id
        WHERE e.revision IS NULL OR e.actor_id IS DISTINCT FROM r.requester_id
            OR e.actor_label IS DISTINCT FROM r.requester_email
            OR ROW(e.at_seconds,e.at_nanoseconds)
                IS DISTINCT FROM ROW(r.created_at_seconds,r.created_at_nanoseconds)
            OR d.case_id IS DISTINCT FROM r.case_id OR d.digest IS DISTINCT FROM r.content_digest
            OR cb.deployment_id IS DISTINCT FROM r.deployment_id
            OR fb.deployment_id IS DISTINCT FROM r.deployment_id
            OR ROW(cb.registered_at_seconds,cb.registered_at_nanoseconds)
                >ROW(r.created_at_seconds,r.created_at_nanoseconds)
            OR ROW(fb.registered_at_seconds,fb.registered_at_nanoseconds)
                >ROW(r.created_at_seconds,r.created_at_nanoseconds)
            OR r.statement IS DISTINCT FROM contract_request_bytes(r.deployment_id,r.request_id,
                r.case_id,r.document_id,r.document_version,r.content_digest,r.client_id,
                r.firm_signer_id,r.requester_id,r.client_leaf,r.firm_leaf,r.created_at_seconds,
                r.expires_at_seconds)
            OR (d.evidence IS NOT NULL AND EXISTS(SELECT 1 FROM contract_events c
                WHERE c.request_id=r.request_id AND c.kind='completed')))";

const SIGNATURES: &str = "SELECT EXISTS(SELECT 1 FROM contract_party_signatures s
        JOIN contract_signature_requests r ON r.request_id=s.request_id
        LEFT JOIN contract_events e ON e.operation_id=s.operation_id
        LEFT JOIN contract_party_signatures f ON f.request_id=s.request_id AND f.\"position\"=1
        LEFT JOIN document_signing_bindings b ON b.binding_id=s.binding_id
        LEFT JOIN participant_credential_trust_revisions t
            ON t.deployment_id=s.deployment_id AND t.revision=s.trust_revision
        LEFT JOIN participant_credential_authority authority ON authority.deployment_id=s.deployment_id
        WHERE e.request_id IS DISTINCT FROM s.request_id
            OR e.kind IS DISTINCT FROM (CASE s.\"position\" WHEN 1 THEN 'client_signed' ELSE 'completed' END)
            OR e.actor_id IS DISTINCT FROM s.signer_id OR e.actor_label IS DISTINCT FROM s.actor_email
            OR ROW(e.at_seconds,e.at_nanoseconds)
                IS DISTINCT FROM ROW(s.accepted_at_seconds,s.accepted_at_nanoseconds)
            OR (s.\"position\"=1 AND ROW(s.signer_id,s.binding_id,s.leaf_fingerprint)
                IS DISTINCT FROM ROW(r.client_id,r.client_binding_id,r.client_leaf))
            OR (s.\"position\"=2 AND ROW(s.signer_id,s.binding_id,s.leaf_fingerprint)
                IS DISTINCT FROM ROW(r.firm_signer_id,r.firm_binding_id,r.firm_leaf))
            OR s.deployment_id IS DISTINCT FROM r.deployment_id
            OR s.accepted_at_seconds>=r.expires_at_seconds
            OR ROW(s.accepted_at_seconds,s.accepted_at_nanoseconds)
                <ROW(r.created_at_seconds,r.created_at_nanoseconds)
            OR s.root_fingerprint IS DISTINCT FROM authority.root_fingerprint
            OR b.root_fingerprint IS DISTINCT FROM s.root_fingerprint
            OR f.root_fingerprint IS DISTINCT FROM s.root_fingerprint
            OR b.account_revision>s.account_revision OR b.auth_generation>s.auth_generation
            OR b.trust_revision>s.trust_revision
            OR ROW(b.registered_at_seconds,b.registered_at_nanoseconds)
                >ROW(s.accepted_at_seconds,s.accepted_at_nanoseconds)
            OR EXISTS(SELECT 1 FROM document_signing_withdrawals w WHERE w.binding_id=s.binding_id
                AND ROW(w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds)
                    <ROW(s.accepted_at_seconds,s.accepted_at_nanoseconds))
            OR s.valid_from IS DISTINCT FROM greatest(b.certificate_not_before,t.valid_from)
            OR s.valid_until IS DISTINCT FROM least(b.certificate_not_after,t.valid_until)
            OR ROW(s.accepted_at_seconds,s.accepted_at_nanoseconds)
                <ROW(t.published_at_seconds,t.published_at_nanoseconds)
            OR s.statement IS DISTINCT FROM contract_party_bytes(s.deployment_id,s.root_fingerprint,
                s.trust_revision,s.signer_id,s.account_revision,s.auth_generation,s.binding_id,
                s.leaf_fingerprint,s.operation_id,s.request_id,r.statement_digest,r.content_digest,
                s.\"position\",
                CASE s.\"position\" WHEN 1 THEN decode(repeat('00',32),'hex') ELSE sha256(f.signature) END,
                CASE s.\"position\" WHEN 1 THEN decode(repeat('00',32),'hex')
                    ELSE sha256(f.timestamp_token) END))";

const PAIRS: &str = "SELECT EXISTS(SELECT 1 FROM contract_signature_requests r
        WHERE NOT EXISTS(SELECT 1 FROM contract_events e
            WHERE e.request_id=r.request_id AND e.kind='requested'))
    OR EXISTS(SELECT 1 FROM contract_events e WHERE e.kind='client_signed' AND NOT EXISTS(
        SELECT 1 FROM contract_party_signatures s
        WHERE s.operation_id=e.operation_id AND s.\"position\"=1))
    OR EXISTS(SELECT 1 FROM contract_events e WHERE e.kind='completed' AND (NOT EXISTS(
            SELECT 1 FROM contract_party_signatures s
            WHERE s.operation_id=e.operation_id AND s.\"position\"=2)
        OR NOT EXISTS(SELECT 1 FROM contract_completions c WHERE c.request_id=e.request_id)))
    OR EXISTS(SELECT 1 FROM contract_completions c
        LEFT JOIN contract_party_signatures s ON s.request_id=c.request_id AND s.\"position\"=2
        WHERE ROW(s.accepted_at_seconds,s.accepted_at_nanoseconds)
            IS DISTINCT FROM ROW(c.completed_at_seconds,c.completed_at_nanoseconds))
    OR EXISTS(SELECT 1 FROM contract_notifications n
        LEFT JOIN contract_events e ON e.contract_id=n.contract_id AND e.revision=n.event_revision
        WHERE e.request_id IS DISTINCT FROM n.request_id OR e.kind IS DISTINCT FROM n.kind
            OR e.at_seconds IS DISTINCT FROM n.created_at_seconds)";

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        // A CHECK admits NULL, so only a false predicate is a violation.
        let violated = checks::expected(table)
            .iter()
            .map(|predicate| format!("({predicate}) IS FALSE"))
            .collect::<Vec<_>>()
            .join(" OR ");
        reject(
            client,
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {violated})"),
        )?;
    }
    for query in [ROOTS, AUDIT, REQUESTS, SIGNATURES, PAIRS] {
        reject(client, query)?;
    }
    Ok(())
}

fn reject<C: GenericClient>(client: &mut C, query: &str) -> Result<(), ApplicationError> {
    if client
        .query_one(query, &[])
        .map_err(port)?
        .get::<_, bool>(0)
    {
        return Err(inconsistent());
    }
    Ok(())
}
