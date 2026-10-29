use super::{checks, incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

/// One cutoff; receipts at or after it have exactly one manifest and older
/// receipts none; each manifest matches its receipt and document version.
const PAIRS: &str = "SELECT (SELECT count(*) FROM individual_manifest_cutoff)<>1
    OR EXISTS(SELECT 1 FROM individual_document_signatures r
        CROSS JOIN individual_manifest_cutoff c
        LEFT JOIN individual_document_manifests m ON m.operation_id=r.operation_id
        WHERE (r.audit_sequence>=c.first_audit_sequence) IS DISTINCT FROM (m.operation_id IS NOT NULL))
    OR EXISTS(SELECT 1 FROM individual_document_manifests m
        LEFT JOIN individual_document_signatures r ON r.operation_id=m.operation_id
        LEFT JOIN documents d ON d.id=m.document_id AND d.version=m.document_version
        WHERE r.operation_id IS NULL OR d.id IS NULL
            OR r.document_id IS DISTINCT FROM m.document_id
            OR r.document_version IS DISTINCT FROM m.document_version
            OR r.content_digest IS DISTINCT FROM m.document_sha256
            OR d.digest IS DISTINCT FROM m.document_sha256)";

/// Checks stored rows against the catalog rules and their receipts; the
/// store then parses each manifest and verifies its token.
pub(crate) fn validate_inventory<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let mut queries = Vec::new();
    for table in TABLES {
        let predicates = checks::expected(table).join(" AND ");
        queries.push(format!(
            "SELECT EXISTS(SELECT 1 FROM {table} WHERE ({predicates}) IS NOT TRUE)"
        ));
    }
    queries.push(PAIRS.to_owned());
    for query in &queries {
        if client
            .query_one(query.as_str(), &[])
            .map_err(port)?
            .get::<_, bool>(0)
        {
            return Err(incomplete());
        }
    }
    Ok(())
}
