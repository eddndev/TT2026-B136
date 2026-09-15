use super::{decode::digest, inconsistent, port};
use application::case_stages::{StageSupportReadLimits, StageSupportRef};
use application::documents::DocumentRecord;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};
use postgres::Transaction;

pub(super) fn require_scope(
    tx: &mut Transaction<'_>,
    case: CaseId,
    supports: &[StageSupportRef],
) -> Result<(), ApplicationError> {
    for support in supports {
        let reference = support.reference();
        tx.query_opt(
            "SELECT 1 FROM documents WHERE id=$1 AND case_id=$2 AND version=$3",
            &[
                &reference.id.as_uuid(),
                &case.as_uuid(),
                &i64::from(reference.version.get()),
            ],
        )
        .map_err(port)?
        .ok_or_else(|| ApplicationError::DocumentNotFound(reference.id.to_string()))?;
    }
    Ok(())
}

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    support: StageSupportRef,
    limits: &StageSupportReadLimits,
) -> Result<DocumentRecord, ApplicationError> {
    let reference = support.reference();
    let id = reference.id.as_uuid();
    let case = case.as_uuid();
    let version = i64::from(reference.version.get());
    let probe=tx.query_opt("SELECT octet_length(vault)::bigint AS vault_size,substring(vault FROM 1 FOR 9) AS header,octet_length(name) AS name_size,octet_length(digest) AS digest_size,COALESCE(octet_length(evidence::text),0)::bigint AS evidence_size FROM documents WHERE id=$1 AND case_id=$2 AND version=$3",&[&id,&case,&version]).map_err(port)?.ok_or_else(||ApplicationError::DocumentNotFound(reference.id.to_string()))?;
    let vault_size = usize::try_from(probe.get::<_, i64>("vault_size")).map_err(inconsistent)?;
    let header: Vec<u8> = probe.get("header");
    limits.vault().inspect_header(&header, vault_size)?;
    let evidence_size =
        usize::try_from(probe.get::<_, i64>("evidence_size")).map_err(inconsistent)?;
    if evidence_size > limits.max_evidence_json_bytes() {
        return Err(ApplicationError::StageSupportTooLarge);
    }
    if probe.get::<_, i32>("name_size") > 128 || probe.get::<_, i32>("digest_size") != 32 {
        return Err(inconsistent("invalid stored support metadata length"));
    }
    let maximum = i64::try_from(limits.vault().max_vault_bytes()).map_err(inconsistent)?;
    let evidence_maximum = i64::try_from(limits.max_evidence_json_bytes()).map_err(inconsistent)?;
    // The final statement repeats every allocation bound in its own snapshot.
    let row=tx.query_opt("SELECT id,version,name,digest,vault,evidence::text AS evidence_text FROM documents WHERE id=$1 AND case_id=$2 AND version=$3 AND octet_length(vault)::bigint BETWEEN 97 AND $4 AND substring(vault FROM 1 FOR 9)=$5 AND octet_length(name)<=128 AND octet_length(digest)=32 AND COALESCE(octet_length(evidence::text),0)::bigint<=$6",&[&id,&case,&version,&maximum,&header,&evidence_maximum]).map_err(port)?.ok_or(ApplicationError::StageSupportChanged)?;
    let stored_digest = digest(row.get("digest"))?;
    if stored_digest != support.digest() {
        return Err(ApplicationError::StageSupportDigestMismatch);
    }
    let mut record = DocumentRecord::pending(
        DocumentId::from_uuid(row.get("id")),
        DocumentVersion::new(u32::try_from(row.get::<_, i64>("version")).map_err(inconsistent)?)
            .map_err(inconsistent)?,
        row.get("name"),
        stored_digest,
        row.get("vault"),
    )?;
    if let Some(text) = row.get::<_, Option<String>>("evidence_text") {
        record.seal(crate::documents::decode_evidence_text(&text)?)?;
    }
    limits.check_record(&record)?;
    Ok(record)
}
