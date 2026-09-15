use application::ApplicationError;
use postgres::Client;

/// Checks structural protection and metadata continuity once when runtime opens.
pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let present: bool = client
        .query_one(
            "SELECT pg_catalog.to_regclass('document_series') IS NOT NULL
         AND pg_catalog.to_regprocedure('preserve_document_series()') IS NOT NULL
         AND pg_catalog.to_regprocedure('enforce_document_version_sequence()') IS NOT NULL",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !present {
        return Err(incomplete());
    }
    let protected: bool = client.query_one(
        "SELECT EXISTS (
             SELECT 1 FROM pg_catalog.pg_constraint c WHERE c.conrelid='documents'::pg_catalog.regclass
             AND c.contype='p' AND (SELECT array_agg(a.attname::text ORDER BY k.ordinality)
                 FROM unnest(c.conkey) WITH ORDINALITY k(attnum,ordinality)
                 JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.attnum
             )=ARRAY['id','version'])
         AND EXISTS (SELECT 1 FROM pg_catalog.pg_constraint WHERE
             conrelid='documents'::pg_catalog.regclass AND conname='documents_series_case_fk'
             AND contype='f' AND convalidated AND confrelid='document_series'::pg_catalog.regclass)
         AND EXISTS (SELECT 1 FROM pg_catalog.pg_constraint WHERE
             conrelid='document_series'::pg_catalog.regclass AND conname='document_series_first_version_fk'
             AND contype='f' AND convalidated AND condeferrable AND condeferred
             AND confrelid='documents'::pg_catalog.regclass)
         AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_trigger t
             JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint
             WHERE t.tgenabled NOT IN ('O','A') AND (
                 (c.conrelid='documents'::pg_catalog.regclass AND c.conname='documents_series_case_fk')
                 OR (c.conrelid='document_series'::pg_catalog.regclass AND c.conname='document_series_first_version_fk')
             ))
         AND (SELECT count(*) FROM pg_catalog.pg_trigger WHERE tgenabled IN ('O','A') AND (
             (tgrelid='document_series'::pg_catalog.regclass AND tgname='document_series_preserve_identity'
                 AND tgfoid='preserve_document_series()'::pg_catalog.regprocedure AND tgtype=19)
             OR (tgrelid='documents'::pg_catalog.regclass AND tgname='documents_version_sequence'
                 AND tgfoid='enforce_document_version_sequence()'::pg_catalog.regprocedure AND tgtype=7)
             OR (tgrelid='documents'::pg_catalog.regclass AND tgname='documents_preserve_evidence'
                 AND tgfoid='preserve_document_evidence()'::pg_catalog.regprocedure AND tgtype=19)
         ))=3",
        &[],
    ).map_err(port)?.get(0);
    if !protected {
        return Err(incomplete());
    }
    Ok(())
}

pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let broken: bool = client.query_one(
        "SELECT EXISTS (
             SELECT s.id FROM document_series s LEFT JOIN documents d
                 ON d.id=s.id AND d.case_id=s.case_id
             GROUP BY s.id,s.first_available_version
             HAVING MIN(d.version) IS DISTINCT FROM s.first_available_version
                 OR COUNT(d.version)<>MAX(d.version)-s.first_available_version+1
         ) OR EXISTS (
             SELECT 1 FROM documents d LEFT JOIN document_series s ON s.id=d.id AND s.case_id=d.case_id
             WHERE s.id IS NULL
         )",
        &[],
    ).map_err(port)?.get(0);
    if broken {
        return Err(ApplicationError::InvalidConfiguration(
            "document version inventory is incomplete; restore a consistent database".into(),
        ));
    }
    Ok(())
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "document version schema is incomplete; run database migrate with an administrative role"
            .into(),
    )
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("document version schema: {error}"))
}
