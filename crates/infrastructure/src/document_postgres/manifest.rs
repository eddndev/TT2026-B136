//! The stored evidence manifest of an individually signed document version,
//! read for export after the caller authorized that exact version.

use super::PostgresCaseDocumentStore;
use application::{
    documents::{DocumentRecord, ManifestExport},
    ApplicationError,
};
use domain::crypto::DocumentVersionRef;

pub(super) fn signing_manifest(
    store: &PostgresCaseDocumentStore,
    record: &DocumentRecord,
) -> Result<Option<ManifestExport>, ApplicationError> {
    let mut client = store.client()?;
    crate::individual_signing_postgres::read_manifest::for_export(
        &mut *client,
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        &record.digest,
    )
}
