use super::{inconsistent, port, PostgresIndividualSigningStore};
use application::{
    document_signing::{
        validate_signing_receipt, SigningReceipt, SigningVerificationContext,
        SigningWorkflowError as Error,
    },
    documents::{DocumentAction, VersionSelection},
    identity::Principal,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, DocumentVersionRef},
    document_signing::DocumentConsent,
    identity::Role,
};
use postgres::{GenericClient, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn authorize(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    case: CaseId,
    reference: DocumentVersionRef,
    signing: bool,
) -> Result<(), ApplicationError> {
    if signing && actor.role != Role::Litigator {
        return Err(ApplicationError::PermissionDenied);
    }
    let current = crate::document_postgres::authorize_signing_document(
        tx,
        actor.id,
        case,
        reference.id,
        if signing {
            DocumentAction::Seal
        } else {
            DocumentAction::Verify
        },
    )?;
    if current != *actor {
        return Err(Error::AccountChanged.into());
    }
    // Case membership alone does not establish a document's immutable case association.
    let exists = tx
        .query_opt(
            "SELECT id FROM documents WHERE id=$1 AND case_id=$2 AND version=$3",
            &[
                &reference.id.as_uuid(),
                &case.as_uuid(),
                &i64::from(reference.version.get()),
            ],
        )
        .map_err(port)?;
    if exists.is_none() {
        return Err(ApplicationError::DocumentNotFound(reference.id.to_string()));
    }
    Ok(())
}

pub(super) fn find(
    store: &PostgresIndividualSigningStore,
    actor: &Principal,
    case: CaseId,
    reference: DocumentVersionRef,
    operation: Uuid,
) -> Result<Option<SigningReceipt>, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    authorize(&mut tx, actor, case, reference, true)?;
    let receipt = load(&mut tx, operation)?;
    let receipt = receipt.filter(|value| {
        value.consent.account().id() == actor.id
            && value.consent.case() == case
            && value.consent.document() == reference
    });
    tx.commit().map_err(port)?;
    Ok(receipt)
}

pub(super) fn verification_context(
    store: &PostgresIndividualSigningStore,
    actor: &Principal,
    case: CaseId,
    reference: DocumentVersionRef,
) -> Result<SigningVerificationContext, ApplicationError> {
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    authorize(&mut tx, actor, case, reference, false)?;
    let record = crate::document_postgres::load_signing_record(
        &mut tx,
        case,
        reference.id,
        VersionSelection::Exact(reference.version),
    )?;
    let mut withdrawn = None;
    let mut signing = None;
    if let Some(proof) = record
        .evidence
        .as_ref()
        .and_then(|value| value.individual.as_ref())
    {
        let consent = DocumentConsent::from_canonical_bytes(&proof.consent_statement)
            .map_err(|_| inconsistent())?;
        if consent.case() != case || consent.document() != reference {
            return Err(inconsistent());
        }
        let receipt = load(&mut tx, consent.operation())?.ok_or_else(inconsistent)?;
        if record.evidence.as_ref() != Some(&receipt.evidence) {
            return Err(inconsistent());
        }
        let binding = crate::signing_binding_postgres::load(&mut tx, consent.material().binding())?
            .ok_or_else(inconsistent)?;
        withdrawn = binding.withdrawn_at;
        signing = Some(receipt);
    }
    let trust = crate::credential_trust_postgres::current(&mut tx)?;
    crate::audit_postgres::append_transaction(
        &mut tx,
        &actor.email,
        "document.verified",
        &format!(
            "case:{case}:document:{}:version:{}:individual-status",
            reference.id,
            reference.version.get()
        ),
        store.clock.now(),
    )?;
    tx.commit().map_err(port)?;
    Ok(SigningVerificationContext {
        signing,
        record,
        trust,
        binding_withdrawn_at: withdrawn,
    })
}

pub(super) fn load<C: GenericClient>(
    client: &mut C,
    operation: Uuid,
) -> Result<Option<SigningReceipt>, ApplicationError> {
    let row = client.query_opt("SELECT r.*,d.evidence FROM individual_document_signatures r
        JOIN documents d ON d.id=r.document_id AND d.version=r.document_version AND d.case_id=r.case_id
        WHERE r.operation_id=$1", &[&operation]).map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    decode(&row).map(Some)
}

pub(super) fn decode(row: &Row) -> Result<SigningReceipt, ApplicationError> {
    let bytes: Vec<u8> = row.try_get("consent").map_err(port)?;
    let consent = DocumentConsent::from_canonical_bytes(&bytes).map_err(|_| inconsistent())?;
    let signature: Vec<u8> = row.try_get("pdf_signature").map_err(port)?;
    let proof: Vec<u8> = row.try_get("consent_signature").map_err(port)?;
    let seconds: i64 = row.try_get("accepted_at_seconds").map_err(port)?;
    let nanos: i32 = row.try_get("accepted_at_nanoseconds").map_err(port)?;
    let accepted_at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| inconsistent())?
        .replace_nanosecond(nanos.try_into().map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())?;
    let receipt = SigningReceipt {
        consent,
        evidence: crate::documents::decode_evidence(row.try_get("evidence").map_err(port)?)?,
        accepted_at,
    };
    let actor = Principal {
        id: consent.account().id(),
        role: Role::Litigator,
        email: row.try_get("actor_email").map_err(port)?,
    };
    validate_signing_receipt(
        &crate::RingSha256Hasher,
        &receipt,
        &actor,
        consent.case(),
        consent.document(),
        consent.operation(),
    )?;
    let material = consent.material();
    let check = |name: &str, expected: &[u8]| -> Result<(), ApplicationError> {
        if row.try_get::<_, Vec<u8>>(name).map_err(port)? != expected {
            return Err(inconsistent());
        }
        Ok(())
    };
    check("root_fingerprint", material.root().as_bytes())?;
    check("leaf_fingerprint", material.leaf().as_bytes())?;
    check("content_digest", consent.content_digest().as_bytes())?;
    check(
        "consent_digest",
        crate::RingSha256Hasher.hash_bytes(&bytes).as_bytes(),
    )?;
    if row.try_get::<_, Uuid>("operation_id").map_err(port)? != consent.operation()
        || row.try_get::<_, Uuid>("case_id").map_err(port)? != consent.case().as_uuid()
        || row.try_get::<_, Uuid>("document_id").map_err(port)? != consent.document().id.as_uuid()
        || row.try_get::<_, i64>("document_version").map_err(port)?
            != i64::from(consent.document().version.get())
        || row.try_get::<_, Uuid>("signer_id").map_err(port)? != consent.account().id().as_uuid()
        || row.try_get::<_, Uuid>("binding_id").map_err(port)? != material.binding()
        || row.try_get::<_, Uuid>("deployment_id").map_err(port)? != material.deployment()
        || row.try_get::<_, i64>("trust_revision").map_err(port)?
            != i64::from(material.trust_revision())
        || row.try_get::<_, i64>("account_revision").map_err(port)?
            != consent.account().revision() as i64
        || row.try_get::<_, i64>("auth_generation").map_err(port)?
            != consent.account().generation() as i64
        || receipt.evidence.signature != signature
        || receipt
            .evidence
            .individual
            .as_ref()
            .ok_or_else(inconsistent)?
            .consent_signature
            != proof
    {
        return Err(inconsistent());
    }
    let checked: i64 = row.try_get("checked_at").map_err(port)?;
    let start: i64 = row.try_get("valid_from").map_err(port)?;
    let end: i64 = row.try_get("valid_until").map_err(port)?;
    if start > checked || checked > seconds || seconds > end {
        return Err(inconsistent());
    }
    Ok(receipt)
}
