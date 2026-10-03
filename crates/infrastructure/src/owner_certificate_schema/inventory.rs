use super::{checks, incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

const REGISTRATION: &str = "SELECT EXISTS(
    SELECT 1 FROM owner_certificate_registrations r
    LEFT JOIN users u ON u.id=r.owner_id
    LEFT JOIN participant_credential_trust_revisions t
        ON t.deployment_id=r.deployment_id AND t.revision=r.trust_revision
    LEFT JOIN participant_credential_authority authority ON authority.deployment_id=r.deployment_id
    LEFT JOIN audit_events a ON a.sequence=r.audit_sequence
    WHERE u.id IS NULL OR t.revision IS NULL OR authority.deployment_id IS NULL OR a.sequence IS NULL
        OR u.revision<r.account_revision OR u.auth_generation<r.auth_generation
        OR r.root_fingerprint IS DISTINCT FROM authority.root_fingerprint
        OR r.valid_from IS DISTINCT FROM greatest(r.certificate_not_before,t.valid_from)
        OR r.valid_until IS DISTINCT FROM least(r.certificate_not_after,t.valid_until)
        OR ROW(r.registered_at_seconds,r.registered_at_nanoseconds)
            <ROW(t.published_at_seconds,t.published_at_nanoseconds)
        OR r.statement IS DISTINCT FROM (convert_to('OWNCERT1','UTF8') || decode('0101','hex')
            || uuid_send(r.deployment_id) || r.root_fingerprint
            || substring(int8send(r.trust_revision) FROM 5 FOR 4)
            || uuid_send(r.owner_id) || int8send(r.account_revision) || int8send(r.auth_generation)
            || uuid_send(r.binding_id) || decode('0000000000000001','hex') || r.leaf_fingerprint)
        OR a.actor IS DISTINCT FROM r.actor_email OR a.action IS DISTINCT FROM 'identity.owner_certificate_registered'
        OR a.resource IS DISTINCT FROM ('owner-certificate:' || r.binding_id::text || ':owner:' || r.owner_id::text
            || ':revision:1:statement:' || encode(r.statement_digest,'hex'))
        OR a.timestamp_seconds IS DISTINCT FROM r.registered_at_seconds
        OR a.timestamp_nanos IS DISTINCT FROM r.registered_at_nanoseconds)";

const WITHDRAWAL: &str = "SELECT EXISTS(
    SELECT 1 FROM owner_certificate_withdrawals w
    LEFT JOIN owner_certificate_registrations r USING(binding_id)
    LEFT JOIN users u ON u.id=r.owner_id
    LEFT JOIN audit_events a ON a.sequence=w.audit_sequence
    WHERE r.binding_id IS NULL OR u.id IS NULL OR a.sequence IS NULL
        OR u.revision<w.account_revision OR u.auth_generation<w.auth_generation
        OR w.account_revision<r.account_revision OR w.auth_generation<r.auth_generation
        OR w.audit_sequence<=r.audit_sequence
        OR ROW(w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds)
            <ROW(r.registered_at_seconds,r.registered_at_nanoseconds)
        OR w.statement IS DISTINCT FROM (convert_to('OWNCERT1','UTF8') || decode('0201','hex')
            || uuid_send(r.deployment_id) || r.root_fingerprint
            || substring(int8send(r.trust_revision) FROM 5 FOR 4)
            || uuid_send(r.owner_id) || int8send(w.account_revision) || int8send(w.auth_generation)
            || uuid_send(w.binding_id) || decode('0000000100000002','hex') || r.leaf_fingerprint)
        OR a.actor IS DISTINCT FROM w.actor_email OR a.action IS DISTINCT FROM 'identity.owner_certificate_withdrawn'
        OR a.resource IS DISTINCT FROM ('owner-certificate:' || w.binding_id::text || ':owner:' || r.owner_id::text
            || ':revision:2:statement:' || encode(w.statement_digest,'hex'))
        OR a.timestamp_seconds IS DISTINCT FROM w.withdrawn_at_seconds
        OR a.timestamp_nanos IS DISTINCT FROM w.withdrawn_at_nanoseconds)";

const HISTORY: &str = "SELECT EXISTS(
    SELECT 1 FROM owner_certificate_registrations GROUP BY leaf_fingerprint HAVING count(DISTINCT owner_id)>1)
    OR EXISTS(SELECT 1 FROM owner_certificate_registrations r
        JOIN owner_certificate_registrations successor ON successor.owner_id=r.owner_id
            AND successor.audit_sequence>r.audit_sequence
        LEFT JOIN owner_certificate_withdrawals w ON w.binding_id=r.binding_id
        WHERE w.binding_id IS NULL OR w.audit_sequence>=successor.audit_sequence
            OR w.account_revision>successor.account_revision OR w.auth_generation>successor.auth_generation
            OR ROW(w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds)
                >ROW(successor.registered_at_seconds,successor.registered_at_nanoseconds))
    OR EXISTS(SELECT 1 FROM audit_events a
        WHERE a.action='identity.owner_certificate_registered' AND NOT EXISTS(
            SELECT 1 FROM owner_certificate_registrations r WHERE r.audit_sequence=a.sequence))
    OR EXISTS(SELECT 1 FROM audit_events a
        WHERE a.action='identity.owner_certificate_withdrawn' AND NOT EXISTS(
            SELECT 1 FROM owner_certificate_withdrawals w WHERE w.audit_sequence=a.sequence))";

/// Checks historical relations without imposing today's account or trust state.
pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let predicates = checks::expected(table).join(" AND ");
        reject(
            client,
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE ({predicates}) IS NOT TRUE)"),
        )?;
    }
    for query in [REGISTRATION, WITHDRAWAL, HISTORY] {
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
        return Err(incomplete());
    }
    Ok(())
}
