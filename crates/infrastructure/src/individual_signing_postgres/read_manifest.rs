//! Loads the sealed evidence manifest stored beside a receipt.

use super::{inconsistent, port};
use application::{
    documents::{ManifestExport, SigningManifest},
    ApplicationError,
};
use domain::{
    crypto::{DocumentVersionRef, Sha256Digest},
    document_signing::DocumentConsent,
};
use postgres::{GenericClient, Row};
use uuid::Uuid;

/// The manifest of the receipt of `consent`, if its acceptance stored one,
/// after checking it names that operation, document version and digest.
///
/// A receipt at or after the manifest cutoff without its manifest, or one
/// before it with a manifest, is inconsistent, so a replay or read never
/// returns a receipt that lost its manifest.
pub(super) fn find<C: GenericClient>(
    client: &mut C,
    consent: &DocumentConsent,
) -> Result<Option<SigningManifest>, ApplicationError> {
    let row = client
        .query_opt(
            &format!(
                "SELECT m.*,{REQUIRES_MANIFEST} FROM individual_document_signatures r
                LEFT JOIN individual_document_manifests m ON m.operation_id=r.operation_id
                WHERE r.operation_id=$1"
            ),
            &[&consent.operation()],
        )
        .map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    if !stored(&row)? {
        return Ok(None);
    }
    decode(
        &row,
        consent.operation(),
        consent.document(),
        &consent.content_digest(),
    )
    .map(Some)
}

/// The manifest and receipt facts of the individual signature accepted for
/// exactly this document version, if its acceptance stored a manifest.
///
/// A receipt at or after the manifest cutoff without its manifest, or one
/// before it with a manifest, is inconsistent rather than exported as a
/// package without the manifest entries.
pub(crate) fn for_export<C: GenericClient>(
    client: &mut C,
    reference: DocumentVersionRef,
    content: &Sha256Digest,
) -> Result<Option<ManifestExport>, ApplicationError> {
    let row = client
        .query_opt(
            &format!(
                "SELECT m.*,r.accepted_at_seconds,r.accepted_at_nanoseconds,{REQUIRES_MANIFEST}
                FROM individual_document_signatures r
                LEFT JOIN individual_document_manifests m ON m.operation_id=r.operation_id
                WHERE r.document_id=$1 AND r.document_version=$2"
            ),
            &[&reference.id.as_uuid(), &i64::from(reference.version.get())],
        )
        .map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    if !stored(&row)? {
        return Ok(None);
    }
    let operation: Uuid = row.try_get("operation_id").map_err(port)?;
    let manifest = decode(&row, operation, reference, content)?;
    let nanoseconds: i32 = row.try_get("accepted_at_nanoseconds").map_err(port)?;
    Ok(Some(ManifestExport {
        operation,
        accepted_at_seconds: row.try_get("accepted_at_seconds").map_err(port)?,
        accepted_at_nanoseconds: u32::try_from(nanoseconds).map_err(|_| inconsistent())?,
        manifest,
    }))
}

/// Whether the receipt `r` of a query is at or after the manifest cutoff.
const REQUIRES_MANIFEST: &str = "r.audit_sequence>=(SELECT c.first_audit_sequence
    FROM individual_manifest_cutoff c WHERE c.singleton) AS requires_manifest";

/// Whether the row joined to its receipt holds the manifest the cutoff
/// requires; a missing required manifest, a manifest the cutoff excludes or
/// a missing cutoff is inconsistent.
fn stored(row: &Row) -> Result<bool, ApplicationError> {
    let requires: Option<bool> = row.try_get("requires_manifest").map_err(port)?;
    let stored: Option<Uuid> = row.try_get("operation_id").map_err(port)?;
    match (requires, stored) {
        (Some(false), None) => Ok(false),
        (Some(true), Some(_)) => Ok(true),
        _ => Err(inconsistent()),
    }
}

pub(super) fn decode(
    row: &Row,
    operation: Uuid,
    reference: DocumentVersionRef,
    content: &Sha256Digest,
) -> Result<SigningManifest, ApplicationError> {
    let manifest = SigningManifest {
        manifest: row.try_get("manifest").map_err(port)?,
        token: row.try_get("manifest_token").map_err(port)?,
        tsa_crl_pem: row.try_get("tsa_crl_pem").map_err(port)?,
    };
    let digest: Vec<u8> = row.try_get("manifest_sha256").map_err(port)?;
    let document: Vec<u8> = row.try_get("document_sha256").map_err(port)?;
    if row.try_get::<_, Uuid>("operation_id").map_err(port)? != operation
        || row.try_get::<_, Uuid>("document_id").map_err(port)? != reference.id.as_uuid()
        || row.try_get::<_, i64>("document_version").map_err(port)?
            != i64::from(reference.version.get())
        || digest != manifest.digest().as_bytes()
        || document != content.as_bytes()
    {
        return Err(inconsistent());
    }
    manifest.check(operation, reference, content)?;
    Ok(manifest)
}
