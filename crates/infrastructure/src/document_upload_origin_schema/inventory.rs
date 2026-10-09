use super::port;
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let invalid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM document_upload_origins o
         LEFT JOIN document_series s ON s.id=o.document_id AND s.case_id=o.case_id
         LEFT JOIN documents d ON d.id=o.document_id AND d.case_id=o.case_id AND d.version=1
         LEFT JOIN users u ON u.id=o.actor_id
         LEFT JOIN audit_events a ON a.sequence=o.audit_sequence
         WHERE s.first_available_version IS DISTINCT FROM 1 OR d.id IS NULL OR u.id IS NULL
            OR a.actor COLLATE \"C\" IS DISTINCT FROM o.actor_email COLLATE \"C\"
            OR a.action COLLATE \"C\" IS DISTINCT FROM 'document.uploaded' COLLATE \"C\"
            OR a.resource COLLATE \"C\" IS DISTINCT FROM ('case:'||o.case_id::text||':document:'
                ||o.document_id::text||':version:1:sha256:'||encode(d.digest,'hex')) COLLATE \"C\"
            OR a.timestamp COLLATE \"C\" IS DISTINCT FROM (
                to_char(to_timestamp(o.recorded_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS')
                ||CASE WHEN o.recorded_at_nanoseconds=0 THEN 'Z'
                    ELSE '.'||rtrim(lpad(o.recorded_at_nanoseconds::text,9,'0'),'0')||'Z' END
            ) COLLATE \"C\")",
        &[],
    ).map_err(port)?.get(0);
    if invalid {
        return Err(ApplicationError::InvalidConfiguration(
            "document upload origins differ from their stored document or audit event".into(),
        ));
    }
    Ok(())
}
