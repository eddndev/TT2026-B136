use super::{inconsistent, port, read, resource};
use application::{documents::DocumentRecord, ApplicationError};
use domain::{
    crypto::{DocumentHasher, Signature, SignatureVerification, SignatureVerifier},
    identity::Role,
};
use postgres::GenericClient;

/// Rechecks immutable public evidence with historical trust, never current CRLs.
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let rows = client.query("SELECT r.*,d.name,d.digest,d.evidence,a.actor,a.action,a.resource,
        a.timestamp_seconds,a.timestamp_nanos FROM individual_document_signatures r
        JOIN documents d ON d.id=r.document_id AND d.version=r.document_version AND d.case_id=r.case_id
        JOIN audit_events a ON a.sequence=r.audit_sequence ORDER BY r.audit_sequence", &[]).map_err(port)?;
    for row in rows {
        let receipt = read::decode(&row)?;
        let consent = receipt.consent;
        let signature = Signature::from_bytes(receipt.evidence.signature.clone())
            .map_err(|_| inconsistent())?;
        if crate::RsaPkcs1Verifier.verify(
            &consent.content_digest(),
            &signature,
            &receipt.evidence.signer_certificate_pem,
        )? != SignatureVerification::Valid
        {
            return Err(inconsistent());
        }
        let record = DocumentRecord::pending(
            consent.document().id,
            consent.document().version,
            row.try_get("name").map_err(port)?,
            consent.content_digest(),
            Vec::new(),
        )?;
        let proof = receipt
            .evidence
            .individual
            .as_ref()
            .ok_or_else(inconsistent)?;
        proof.validate(
            &record,
            &receipt.evidence,
            &crate::RingSha256Hasher,
            &crate::certificates::InternalRsaDeclarationVerifier,
        )?;
        let binding = crate::signing_binding_postgres::load(client, consent.material().binding())?
            .ok_or_else(inconsistent)?;
        if proof.binding_statement != binding.statement.canonical_bytes()
            || proof.binding_signature != binding.check.signature.as_bytes()
            || proof.binding_root_der != binding.trust.inspection.root_der
            || proof.binding_crl_der != binding.trust.inspection.crl_der
            || proof.binding_registered_at != binding.registered_at.unix_timestamp()
            || binding.statement.account().id() != consent.account().id()
            || row.try_get::<_, Vec<u8>>("digest").map_err(port)?
                != consent.content_digest().as_bytes()
            || row.try_get::<_, String>("actor").map_err(port)?
                != row.try_get::<_, String>("actor_email").map_err(port)?
            || row.try_get::<_, String>("action").map_err(port)? != "document.individually_signed"
            || row.try_get::<_, String>("resource").map_err(port)? != resource(&receipt)
            || row.try_get::<_, i64>("timestamp_seconds").map_err(port)?
                != receipt.accepted_at.unix_timestamp()
            || row.try_get::<_, i32>("timestamp_nanos").map_err(port)?
                != receipt.accepted_at.nanosecond() as i32
        {
            return Err(inconsistent());
        }
        let trust = crate::credential_trust_postgres::exact(
            client,
            consent.material().deployment(),
            application::credential_trust::CredentialTrustRevision::new(
                consent.material().trust_revision(),
            )?,
        )?
        .ok_or_else(inconsistent)?;
        let check = domain::crypto::InternalDeclarationVerifier::verify(
            &crate::certificates::InternalRsaDeclarationVerifier,
            &crate::RingSha256Hasher.hash_bytes(&consent.canonical_bytes()),
            &receipt.evidence.signer_certificate_pem,
            &Signature::from_bytes(proof.consent_signature.clone()).map_err(|_| inconsistent())?,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            row.try_get("checked_at").map_err(port)?,
        )
        .map_err(|_| inconsistent())?;
        let captured = domain::crypto::InternalDeclarationVerifier::inspect_trust(
            &crate::certificates::InternalRsaDeclarationVerifier,
            &receipt.evidence.issuer_certificate_pem,
            &receipt.evidence.crl_pem,
            check.checked_at,
        )
        .map_err(|_| inconsistent())?;
        if captured != trust.inspection {
            return Err(inconsistent());
        }
        if check.valid_from != row.try_get::<_, i64>("valid_from").map_err(port)?
            || check.valid_until != row.try_get::<_, i64>("valid_until").map_err(port)?
        {
            return Err(inconsistent());
        }
        let actor = application::identity::Principal {
            id: consent.account().id(),
            role: Role::Litigator,
            email: row.try_get("actor_email").map_err(port)?,
        };
        application::document_signing::validate_signing_receipt(
            &crate::RingSha256Hasher,
            &receipt,
            &actor,
            consent.case(),
            consent.document(),
            consent.operation(),
        )?;
    }
    Ok(())
}
