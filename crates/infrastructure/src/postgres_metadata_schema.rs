use application::ApplicationError;
use postgres::Client;

/// Validates organizational history protection before opening a runtime adapter.
pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let present: bool = client.query_one(
        "SELECT pg_catalog.to_regclass('document_metadata_revisions') IS NOT NULL
         AND pg_catalog.to_regprocedure('document_metadata_bytes(text,text,text[])') IS NOT NULL
         AND pg_catalog.to_regprocedure('document_metadata_is_canonical(text,text,text[])') IS NOT NULL
         AND pg_catalog.to_regprocedure('document_metadata_text_valid(text,integer)') IS NOT NULL
         AND pg_catalog.to_regprocedure('preserve_document_metadata()') IS NOT NULL
         AND pg_catalog.to_regprocedure('enforce_document_metadata_sequence()') IS NOT NULL", &[],
    ).map_err(port)?.get(0);
    if !present {
        return Err(incomplete());
    }
    let protected: bool = client.query_one(
        "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_constraint c
             WHERE c.conrelid='document_metadata_revisions'::pg_catalog.regclass AND c.contype='p'
             AND (SELECT array_agg(a.attname::text ORDER BY k.ordinality)
                 FROM unnest(c.conkey) WITH ORDINALITY k(attnum,ordinality)
                 JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.attnum
             )=ARRAY['document_id','metadata_revision'])
         AND (SELECT count(*) FROM pg_catalog.pg_constraint WHERE
             conrelid='document_metadata_revisions'::pg_catalog.regclass AND convalidated AND (
                 (contype='c' AND conname IN ('document_metadata_revision_range',
                     'document_metadata_canonical','document_metadata_digest'))
                 OR (contype='f' AND conname='document_metadata_series_fk'
                     AND confrelid='document_series'::pg_catalog.regclass)
                 OR (contype='f' AND conname='document_metadata_actor_fk'
                     AND confrelid='users'::pg_catalog.regclass)))=5
         AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_trigger t
             JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint
             WHERE c.conrelid='document_metadata_revisions'::pg_catalog.regclass
                 AND t.tgenabled NOT IN ('O','A'))
         AND (SELECT count(*) FROM pg_catalog.pg_trigger WHERE
             tgrelid='document_metadata_revisions'::pg_catalog.regclass AND tgenabled IN ('O','A') AND (
                 (tgname='document_metadata_sequence' AND tgtype=7
                     AND tgfoid='enforce_document_metadata_sequence()'::pg_catalog.regprocedure)
                 OR (tgname='document_metadata_preserve_history' AND tgtype=19
                     AND tgfoid='preserve_document_metadata()'::pg_catalog.regprocedure)))=2
         AND (SELECT count(*) FROM pg_catalog.pg_proc WHERE NOT prosecdef AND provolatile='i'
             AND oid=ANY(ARRAY['document_metadata_bytes(text,text,text[])'::pg_catalog.regprocedure,
                 'document_metadata_is_canonical(text,text,text[])'::pg_catalog.regprocedure,
                 'document_metadata_text_valid(text,integer)'::pg_catalog.regprocedure]))=3",
        &[],
    ).map_err(port)?.get(0);
    if !protected {
        return Err(incomplete());
    }
    Ok(())
}

/// Checks persisted metadata only; encrypted content and captured evidence stay unloaded.
pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let broken: bool = client.query_one(
        "SELECT EXISTS (SELECT document_id FROM document_metadata_revisions
             GROUP BY document_id HAVING MIN(metadata_revision)<>1
                 OR COUNT(*)<>MAX(metadata_revision))
         OR EXISTS (SELECT 1 FROM document_metadata_revisions m
             LEFT JOIN document_series s ON s.id=m.document_id
             LEFT JOIN users u ON u.id=m.changed_by
             WHERE s.id IS NULL OR u.id IS NULL OR m.metadata_revision NOT BETWEEN 1 AND 4294967295
                 OR CASE WHEN document_metadata_is_canonical(m.document_type,m.classification,m.tags)
                     THEN m.metadata_digest<>pg_catalog.sha256(document_metadata_bytes(m.document_type,m.classification,m.tags))
                     ELSE TRUE END)",
        &[],
    ).map_err(port)?.get(0);
    if broken {
        return Err(ApplicationError::InvalidConfiguration(
            "document metadata inventory is inconsistent; restore a consistent database".into(),
        ));
    }
    Ok(())
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "document metadata schema is incomplete; run database migrate with an administrative role"
            .into(),
    )
}

fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("document metadata schema: {error}"))
}
