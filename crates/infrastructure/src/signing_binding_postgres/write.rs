use super::{account, inconsistent, load, port, resource, PostgresSigningBindingStore};
use application::{
    document_signing::{
        validate_binding_receipt, SigningBindingReceipt, SigningWorkflowError as Error,
        VerifiedSigningBinding,
    },
    ApplicationError,
};
use postgres::Transaction;

impl PostgresSigningBindingStore {
    pub(super) fn register(
        &self,
        command: VerifiedSigningBinding,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        let prepared = command.prepared();
        let actor = prepared.context().principal.id;
        let binding = prepared.statement().material().binding();
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let context = account(&mut tx, actor)?;
        if context.principal != prepared.context().principal {
            return Err(Error::AccountChanged.into());
        }
        if let Some(receipt) = load(&mut tx, binding)? {
            if receipt.statement != *prepared.statement()
                || receipt.check.certificate != *prepared.certificate()
                || receipt.check.signature != command.check().signature
                || receipt.trust != *prepared.trust()
            {
                return Err(Error::Conflict.into());
            }
            tx.commit().map_err(port)?;
            return Ok(receipt);
        }
        if context.account != prepared.context().account {
            return Err(Error::AccountChanged.into());
        }
        let trust = crate::credential_trust_postgres::current(&mut tx)?;
        if trust.as_ref() != Some(prepared.trust()) {
            return Err(Error::TrustChanged.into());
        }
        let claimed: bool = tx.query_one("SELECT EXISTS(SELECT 1 FROM document_signing_bindings WHERE leaf_fingerprint=$1 AND signer_id<>$2)
            OR EXISTS(SELECT 1 FROM document_signing_bindings r WHERE signer_id=$2
                AND NOT EXISTS(SELECT 1 FROM document_signing_withdrawals w WHERE w.binding_id=r.binding_id))",
            &[&&prepared.certificate().fingerprint.as_bytes()[..], &actor.as_uuid()]).map_err(port)?.get(0);
        if claimed {
            return Err(Error::Conflict.into());
        }
        let at = self.clock.now();
        if at < command.not_before() || at.unix_timestamp() < command.check().checked_at {
            return Err(inconsistent());
        }
        if at.unix_timestamp() > command.check().valid_until {
            return Err(Error::Credential(domain::crypto::CredentialFailure::Expired).into());
        }
        let receipt = SigningBindingReceipt {
            statement: *prepared.statement(),
            check: command.check().clone(),
            trust: prepared.trust().clone(),
            registered_at: at,
            withdrawn_at: None,
        };
        validate_binding_receipt(&crate::RingSha256Hasher, &receipt, actor, binding)?;
        let event = crate::audit_postgres::append_transaction(
            &mut tx,
            &context.principal.email,
            "document.signer_bound",
            &resource(&receipt, false),
            at,
        )?;
        insert(
            &mut tx,
            &receipt,
            &context.principal.email,
            event.event.sequence,
        )?;
        tx.commit().map_err(port)?;
        Ok(receipt)
    }
}
fn insert(
    tx: &mut Transaction<'_>,
    receipt: &SigningBindingReceipt,
    email: &str,
    sequence: u64,
) -> Result<(), ApplicationError> {
    let statement = &receipt.statement;
    let material = statement.material();
    let account = statement.account();
    let check = &receipt.check;
    let summary = &check.certificate.summary;
    let bytes = statement.canonical_bytes();
    let sequence = i64::try_from(sequence).map_err(|_| inconsistent())?;
    tx.execute("INSERT INTO document_signing_bindings(binding_id,signer_id,account_revision,auth_generation,actor_email,
        deployment_id,trust_revision,root_fingerprint,leaf_fingerprint,statement,statement_digest,certificate_der,signature,
        certificate_subject,certificate_issuer,certificate_serial,certificate_not_before,certificate_not_after,
        checked_at,valid_from,valid_until,registered_at_seconds,registered_at_nanoseconds,audit_sequence)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
        &[&material.binding(), &account.id().as_uuid(), &(account.revision() as i64), &(account.generation() as i64), &email,
            &material.deployment(), &i64::from(material.trust_revision()), &&material.root().as_bytes()[..], &&material.leaf().as_bytes()[..],
            &&bytes[..], &&check.statement_digest.as_bytes()[..], &check.certificate.der, &check.signature.as_bytes(),
            &summary.subject, &summary.issuer, &summary.serial_hex, &summary.not_before_unix, &summary.not_after_unix,
            &check.checked_at, &check.valid_from, &check.valid_until, &receipt.registered_at.unix_timestamp(),
            &(receipt.registered_at.nanosecond() as i32), &sequence]).map_err(port)?;
    Ok(())
}
