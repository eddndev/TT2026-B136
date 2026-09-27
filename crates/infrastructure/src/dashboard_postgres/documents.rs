use super::{capacity, inconsistent, port, MAX_DOCUMENTS};
use application::ApplicationError;
use domain::crypto::DocumentVersion;
use postgres::Transaction;
use uuid::Uuid;

pub(super) fn pending_contracts(
    tx: &mut Transaction<'_>,
    cases: &[Uuid],
) -> Result<u64, ApplicationError> {
    let rows = tx
        .query(
            "SELECT s.id,d.version,(d.evidence IS NOT NULL) AS sealed,
            m.metadata_revision,m.document_type,m.classification,m.tags,m.metadata_digest,
            m.changed_at,m.changed_by,m.changed_by_email
         FROM document_series s LEFT JOIN LATERAL (
            SELECT version,evidence FROM documents WHERE id=s.id AND case_id=s.case_id
            ORDER BY version DESC LIMIT 1
         ) d ON true LEFT JOIN LATERAL (
            SELECT metadata_revision,document_type,classification,tags,metadata_digest,
                changed_at,changed_by,changed_by_email
            FROM document_metadata_revisions WHERE document_id=s.id
            ORDER BY metadata_revision DESC LIMIT 1
         ) m ON true
         WHERE s.case_id=ANY($1::uuid[]) ORDER BY s.id LIMIT $2",
            &[&cases, &((MAX_DOCUMENTS + 1) as i64)],
        )
        .map_err(port)?;
    capacity(rows.len(), MAX_DOCUMENTS, "document")?;
    let mut count = 0;
    for row in rows {
        let raw: Option<i64> = row.try_get("version").map_err(port)?;
        let raw = raw.ok_or_else(|| inconsistent("document root has no content head"))?;
        DocumentVersion::new(u32::try_from(raw).map_err(inconsistent)?).map_err(inconsistent)?;
        let metadata = crate::document_postgres::current_metadata_row(&row)?;
        let contract = metadata.values.document_type().is_some_and(|value| {
            value.trim().eq_ignore_ascii_case("contrato")
                || value.trim().eq_ignore_ascii_case("contract")
        });
        let sealed: bool = row.try_get("sealed").map_err(port)?;
        if contract && !sealed {
            count += 1;
        }
    }
    Ok(count)
}
