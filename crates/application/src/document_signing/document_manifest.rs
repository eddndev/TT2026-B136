//! Builds and time-stamps the evidence manifest of an accepted individual
//! signature from the final sealed bytes.
//!
//! Rationale: the sections "The evidence manifest" and "Generation inside
//! the acceptance operation" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use domain::{crypto::Sha256Digest, document_signing::DocumentConsent};
use evidence_verify::manifest::{
    self, role_entry, Artifact, Manifest, Role, Signer, SignerEntries, SIGNER_ID,
};

use super::{
    document_validation as validation, DocumentSigningPorts, SigningWorkflowError as Error,
    TokenFacts, TsaAuthority,
};
use crate::{
    credential_trust::CredentialTrustSnapshot,
    documents::{DocumentRecord, IndividualSigningEvidence, SigningManifest},
    ApplicationError,
};

/// Rejects a time-stamping anchor other than the credential trust root,
/// because that root's CRL is the only time-stamping CRL source.
pub(super) fn check_anchor(
    anchor_pem: &[u8],
    trust: &CredentialTrustSnapshot,
) -> Result<(), ApplicationError> {
    match evidence_verify::pem::decode_one(anchor_pem, "CERTIFICATE") {
        Ok(der) if der == trust.inspection.root_der => Ok(()),
        _ => Err(ApplicationError::InvalidConfiguration(
            "individual signing requires the time-stamping anchor to be the credential trust root"
                .into(),
        )),
    }
}

/// The manifest bytes, its verified token and the captured CRL, plus the
/// manifest token facts the caller still compares with its own clock.
pub(super) struct Generated {
    pub(super) manifest: SigningManifest,
    pub(super) facts: TokenFacts,
}

/// Requests the manifest token after the document token; no step retries.
pub(super) fn generate(
    ports: &DocumentSigningPorts,
    trust: &CredentialTrustSnapshot,
    record: &DocumentRecord,
    size: u64,
    individual: &IndividualSigningEvidence,
    consent: &DocumentConsent,
) -> Result<Generated, ApplicationError> {
    let evidence = record.evidence.as_ref().ok_or(Error::Inconsistent)?;
    let anchor = evidence
        .tsa_chain_pem
        .as_deref()
        .ok_or(Error::Inconsistent)?;
    let document = ports
        .manifest_timestamps
        .facts(&evidence.timestamp_token, &record.digest)?;
    let tsa_crl_pem = validation::pem("X509 CRL", &trust.inspection.crl_der);
    let bytes = encode(record, size, anchor, &tsa_crl_pem, individual, consent)?;
    let digest = Sha256Digest::from_array(evidence_verify::certificate::sha256(&bytes));
    let facts = ports.manifest_timestamps.stamp(
        &digest,
        anchor,
        &TsaAuthority {
            root_der: &trust.inspection.root_der,
            crl_der: &trust.inspection.crl_der,
        },
    )?;
    // The authority does not guarantee ordering between two tokens.
    if facts.gen_time < document.gen_time {
        return Err(ApplicationError::TimestampEvidenceRejected(
            "manifest token genTime precedes the document token genTime".into(),
        ));
    }
    Ok(Generated {
        manifest: SigningManifest {
            manifest: bytes,
            token: facts.token.clone(),
            tsa_crl_pem,
        },
        facts,
    })
}

/// Rejects a manifest token that would read as issued after the acceptance
/// instant `accepted_at`, beyond its accuracy.
pub(super) fn check_before(facts: &TokenFacts, accepted_at: i64) -> Result<(), ApplicationError> {
    if facts.gen_time.saturating_sub(facts.accuracy_seconds) > accepted_at {
        return Err(ApplicationError::TimestampEvidenceRejected(
            "manifest token genTime follows the acceptance instant".into(),
        ));
    }
    Ok(())
}

fn encode(
    record: &DocumentRecord,
    size: u64,
    tsa_chain_pem: &[u8],
    tsa_crl_pem: &[u8],
    individual: &IndividualSigningEvidence,
    consent: &DocumentConsent,
) -> Result<Vec<u8>, ApplicationError> {
    let evidence = record.evidence.as_ref().ok_or(Error::Inconsistent)?;
    let name = record.name.as_str();
    let artifact = |role, bytes: &[u8]| Manifest::artifact(name, role, bytes);
    let artifacts = vec![
        Artifact {
            entry: role_entry(Role::Document, name),
            role: Role::Document,
            sha256: *record.digest.as_bytes(),
            size,
        },
        artifact(Role::DocumentSignature, &evidence.signature),
        artifact(Role::DocumentTimestamp, &evidence.timestamp_token),
        artifact(Role::SignerCertificate, &evidence.signer_certificate_pem),
        artifact(Role::IssuerCertificate, &evidence.issuer_certificate_pem),
        artifact(Role::IssuerCrl, &evidence.crl_pem),
        artifact(Role::TsaChain, tsa_chain_pem),
        artifact(Role::TsaIssuerCrl, tsa_crl_pem),
        artifact(Role::BindingStatement, &individual.binding_statement),
        artifact(Role::BindingSignature, &individual.binding_signature),
        artifact(Role::BindingRoot, &individual.binding_root_der),
        artifact(Role::BindingCrl, &individual.binding_crl_der),
        artifact(Role::ConsentStatement, &individual.consent_statement),
        artifact(Role::ConsentSignature, &individual.consent_signature),
    ];
    let material = consent.material();
    let value = Manifest {
        deployment_id: material.deployment().to_string(),
        case_id: consent.case().as_uuid().to_string(),
        document_id: consent.document().id.as_uuid().to_string(),
        document_version: u64::from(consent.document().version.get()),
        document_sha256: *record.digest.as_bytes(),
        operation_id: consent.operation().to_string(),
        artifacts,
        signers: vec![Signer {
            id: SIGNER_ID.into(),
            account_id: consent.account().id().as_uuid().to_string(),
            binding_id: material.binding().to_string(),
            binding_registered_at: individual.binding_registered_at,
            entries: SignerEntries::for_document(name),
        }],
    };
    manifest::encode(&value).map_err(|_| Error::Inconsistent.into())
}
