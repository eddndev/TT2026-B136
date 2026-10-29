use super::{inconsistent, port, read, resource, PostgresIndividualSigningStore};
use application::{
    document_signing::{
        validate_signing_receipt, SigningCommit, SigningReceipt, SigningWorkflowError as Error,
        VerifiedDocumentSigning,
    },
    documents::VersionSelection,
    ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn commit(
    store: &PostgresIndividualSigningStore,
    command: VerifiedDocumentSigning,
) -> Result<SigningCommit, ApplicationError> {
    let consent = *command.consent();
    let mut candidate = SigningReceipt {
        consent,
        evidence: command.record().evidence.clone().ok_or_else(inconsistent)?,
        accepted_at: command.not_before(),
        manifest: Some(command.manifest().clone()),
    };
    validate_signing_receipt(
        &crate::RingSha256Hasher,
        &candidate,
        command.principal(),
        consent.case(),
        consent.document(),
        consent.operation(),
    )?;
    let mut client = store.client()?;
    let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
    let account = crate::signing_binding_postgres::account(&mut tx, command.principal().id)?;
    if account.principal != *command.principal() {
        return Err(Error::AccountChanged.into());
    }
    read::authorize(
        &mut tx,
        command.principal(),
        consent.case(),
        consent.document(),
        true,
    )?;
    if let Some(receipt) = read::load(&mut tx, consent.operation())? {
        if receipt.consent != consent
            || receipt.evidence.signature != candidate.evidence.signature
            || receipt
                .evidence
                .individual
                .as_ref()
                .ok_or_else(inconsistent)?
                .consent_signature
                != candidate
                    .evidence
                    .individual
                    .as_ref()
                    .ok_or_else(inconsistent)?
                    .consent_signature
        {
            return Err(Error::Conflict.into());
        }
        tx.commit().map_err(port)?;
        return Ok(SigningCommit::Existing(receipt));
    }
    if account.account != *command.account() {
        return Err(Error::AccountChanged.into());
    }
    crate::postgres_case_status::require_active(&mut tx, consent.case())?;
    let binding = crate::signing_binding_postgres::load(&mut tx, consent.material().binding())?
        .ok_or(Error::InactiveBinding)?;
    if binding.withdrawn_at.is_some() {
        return Err(Error::InactiveBinding.into());
    }
    if binding != *command.binding() {
        return Err(Error::Conflict.into());
    }
    let trust = crate::credential_trust_postgres::current(&mut tx)?;
    if trust.as_ref() != Some(command.trust()) {
        return Err(Error::TrustChanged.into());
    }
    let current = crate::document_postgres::load_signing_record(
        &mut tx,
        consent.case(),
        consent.document().id,
        VersionSelection::Exact(consent.document().version),
    )?;
    if current.is_sealed() {
        return Err(ApplicationError::DocumentAlreadySealed(
            current.id.to_string(),
        ));
    }
    crate::document_postgres::unchanged_signing_record(&current, command.record())?;
    let at = store.clock.now();
    if at < command.not_before()
        || at < command.trust().published_at
        || at < binding.registered_at
        || at.unix_timestamp() < command.check().valid_from
        || at.unix_timestamp() > command.check().valid_until
    {
        return Err(Error::Inconsistent.into());
    }
    candidate.accepted_at = at;
    let proof = candidate
        .evidence
        .individual
        .as_mut()
        .ok_or_else(inconsistent)?;
    proof.accepted_at = at.unix_timestamp();
    validate_signing_receipt(
        &crate::RingSha256Hasher,
        &candidate,
        command.principal(),
        consent.case(),
        consent.document(),
        consent.operation(),
    )?;
    candidate
        .evidence
        .individual
        .as_ref()
        .ok_or_else(inconsistent)?
        .validate(
            command.record(),
            &candidate.evidence,
            &crate::RingSha256Hasher,
            &crate::certificates::InternalRsaDeclarationVerifier,
        )?;
    let json = crate::documents::encode_evidence(&candidate.evidence)?;
    let changed = tx.execute("UPDATE documents SET evidence=$1 WHERE id=$2 AND case_id=$3 AND version=$4 AND evidence IS NULL",
        &[&json, &consent.document().id.as_uuid(), &consent.case().as_uuid(),
            &i64::from(consent.document().version.get())]).map_err(port)?;
    if changed != 1 {
        return Err(ApplicationError::ConcurrentModification);
    }
    let event = crate::audit_postgres::append_transaction(
        &mut tx,
        &command.principal().email,
        "document.individually_signed",
        &resource(&candidate),
        at,
    )?;
    insert(&mut tx, &command, &candidate, event.event.sequence)?;
    super::write_manifest::insert(&mut tx, &candidate)?;
    tx.commit().map_err(port)?;
    Ok(SigningCommit::Applied(candidate))
}

fn insert(
    tx: &mut Transaction<'_>,
    command: &VerifiedDocumentSigning,
    receipt: &SigningReceipt,
    audit_sequence: u64,
) -> Result<(), ApplicationError> {
    let consent = receipt.consent;
    let bytes = consent.canonical_bytes();
    let digest = crate::RingSha256Hasher.hash_bytes(&bytes);
    let material = consent.material();
    let check = command.check();
    let sequence = i64::try_from(audit_sequence).map_err(|_| inconsistent())?;
    tx.execute("INSERT INTO individual_document_signatures(
        operation_id,case_id,document_id,document_version,signer_id,binding_id,deployment_id,trust_revision,
        root_fingerprint,leaf_fingerprint,account_revision,auth_generation,actor_email,consent,consent_digest,
        content_digest,pdf_signature,consent_signature,checked_at,valid_from,valid_until,
        accepted_at_seconds,accepted_at_nanoseconds,audit_sequence)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
        &[&consent.operation(), &consent.case().as_uuid(), &consent.document().id.as_uuid(),
            &i64::from(consent.document().version.get()), &consent.account().id().as_uuid(),
            &material.binding(), &material.deployment(), &i64::from(material.trust_revision()),
            &&material.root().as_bytes()[..], &&material.leaf().as_bytes()[..],
            &(consent.account().revision() as i64), &(consent.account().generation() as i64),
            &command.principal().email, &&bytes[..], &&digest.as_bytes()[..],
            &&consent.content_digest().as_bytes()[..], &receipt.evidence.signature,
            &check.signature.as_bytes(), &check.checked_at, &check.valid_from, &check.valid_until,
            &receipt.accepted_at.unix_timestamp(), &(receipt.accepted_at.nanosecond() as i32), &sequence]).map_err(port)?;
    Ok(())
}
