//! Rechecks every stored evidence manifest against the stored evidence it
//! describes, its token and its captured time-stamping CRL.

use super::{inconsistent, port, read_manifest};
use application::{documents::SealedEvidence, ApplicationError};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use evidence_verify::{
    certificate::sha256,
    crl,
    manifest::{role_entry, Manifest, Role},
    pem,
    policy::DEFAULT_ADMITTED_POLICY,
    timestamp,
};
use postgres::{GenericClient, Row};
use uuid::Uuid;

/// For each manifest: the canonical bytes parse and name the receipt's
/// operation, document version and digest; every artifact digest is
/// recomputed from the stored document evidence and the captured CRL; the
/// token verifies over the manifest digest; the CRL decodes and parses.
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client
        .query(
            "SELECT m.*,d.name,d.digest,d.evidence FROM individual_document_manifests m
            JOIN individual_document_signatures r ON r.operation_id=m.operation_id
            JOIN documents d ON d.id=r.document_id AND d.version=r.document_version
            ORDER BY r.audit_sequence",
            &[],
        )
        .map_err(port)?;
    for row in rows {
        check(&row)?;
    }
    Ok(())
}

fn check(row: &Row) -> Result<(), ApplicationError> {
    let operation: Uuid = row.try_get("operation_id").map_err(port)?;
    let version = u32::try_from(row.try_get::<_, i64>("document_version").map_err(port)?)
        .map_err(|_| inconsistent())?;
    let reference = DocumentVersionRef {
        id: DocumentId::from_uuid(row.try_get("document_id").map_err(port)?),
        version: DocumentVersion::new(version).map_err(|_| inconsistent())?,
    };
    let digest: Vec<u8> = row.try_get("digest").map_err(port)?;
    let content = Sha256Digest::from_array(digest.try_into().map_err(|_| inconsistent())?);
    let stored = read_manifest::decode(row, operation, reference, &content)?;
    let parsed = stored.check(operation, reference, &content)?;
    let evidence = crate::documents::decode_evidence(row.try_get("evidence").map_err(port)?)?;
    let name: String = row.try_get("name").map_err(port)?;
    covers(&parsed, &evidence, &name, &content, &stored.tsa_crl_pem)?;
    timestamp::verify(
        &stored.token,
        stored.digest().as_bytes(),
        &[DEFAULT_ADMITTED_POLICY],
    )
    .map_err(|_| inconsistent())?;
    let der = pem::decode_one(&stored.tsa_crl_pem, "X509 CRL").map_err(|_| inconsistent())?;
    crl::parse(&der).map_err(|_| inconsistent())?;
    Ok(())
}

fn covers(
    parsed: &Manifest,
    evidence: &SealedEvidence,
    name: &str,
    content: &Sha256Digest,
    tsa_crl_pem: &[u8],
) -> Result<(), ApplicationError> {
    let individual = evidence.individual.as_ref().ok_or_else(inconsistent)?;
    let chain = evidence.tsa_chain_pem.as_deref().ok_or_else(inconsistent)?;
    if parsed.artifacts.len() != Role::ALL.len() {
        return Err(inconsistent());
    }
    for artifact in &parsed.artifacts {
        let bytes: &[u8] = match artifact.role {
            Role::Document => {
                if artifact.entry != name || artifact.sha256 != *content.as_bytes() {
                    return Err(inconsistent());
                }
                continue;
            }
            Role::DocumentSignature => &evidence.signature,
            Role::DocumentTimestamp => &evidence.timestamp_token,
            Role::SignerCertificate => &evidence.signer_certificate_pem,
            Role::IssuerCertificate => &evidence.issuer_certificate_pem,
            Role::IssuerCrl => &evidence.crl_pem,
            Role::TsaChain => chain,
            Role::TsaIssuerCrl => tsa_crl_pem,
            Role::BindingStatement => &individual.binding_statement,
            Role::BindingSignature => &individual.binding_signature,
            Role::BindingRoot => &individual.binding_root_der,
            Role::BindingCrl => &individual.binding_crl_der,
            Role::ConsentStatement => &individual.consent_statement,
            Role::ConsentSignature => &individual.consent_signature,
        };
        if artifact.entry != role_entry(artifact.role, name)
            || artifact.sha256 != sha256(bytes)
            || u64::try_from(bytes.len()).ok() != Some(artifact.size)
        {
            return Err(inconsistent());
        }
    }
    Ok(())
}
