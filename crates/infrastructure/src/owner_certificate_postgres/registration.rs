use application::{identity::owner_certificates::*, ApplicationError};
use domain::owner_certificates::BindingRecord;
use postgres::Transaction;

use super::{
    account, inconsistent, port, query, storage, validation, PostgresOwnerCertificateStore,
};

impl PostgresOwnerCertificateStore {
    pub(super) fn register(
        &self,
        command: VerifiedOwnerRegistration,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        let prepared = command.registration();
        validation::prepared(prepared)?;
        let actor = prepared.account().principal.id;
        let binding = prepared.statement().material().binding();
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        let account = account::owner(&mut tx, actor)?;
        // UUID reconciliation precedes mutable counter/trust/clock comparisons.
        if let Some(receipt) = query::load(&mut tx, binding, None)? {
            validation::intent(&receipt, prepared, &command.check().signature)?;
            tx.commit().map_err(port)?;
            return Ok(OwnerBindingCommit::Existing(receipt));
        }
        if account != *prepared.account() {
            return Err(OwnerCertificateError::AccountChanged.into());
        }
        let current = crate::credential_trust_postgres::current(&mut tx).map_err(|_| storage())?;
        if current.as_ref() != Some(prepared.trust()) {
            return Err(OwnerCertificateError::TrustChanged.into());
        }
        let fingerprint = prepared.certificate().fingerprint;
        let foreign: bool = tx
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM owner_certificate_registrations
            WHERE leaf_fingerprint=$1 AND owner_id<>$2)",
                &[&&fingerprint.as_bytes()[..], &actor.as_uuid()],
            )
            .map_err(port)?
            .get(0);
        if foreign {
            return Err(OwnerCertificateError::FingerprintConflict.into());
        }
        let live: bool = tx.query_one("SELECT EXISTS(SELECT 1 FROM owner_certificate_registrations r
            WHERE owner_id=$1 AND NOT EXISTS(SELECT 1 FROM owner_certificate_withdrawals w WHERE w.binding_id=r.binding_id))",
            &[&actor.as_uuid()]).map_err(port)?.get(0);
        if live {
            return Err(OwnerCertificateError::ActiveBinding.into());
        }
        let previous = tx.query_opt("SELECT w.withdrawn_at_seconds,w.withdrawn_at_nanoseconds
            FROM owner_certificate_withdrawals w JOIN owner_certificate_registrations r USING(binding_id)
            WHERE r.owner_id=$1 ORDER BY w.audit_sequence DESC LIMIT 1", &[&actor.as_uuid()])
            .map_err(port)?;
        let previous = previous
            .map(|row| {
                super::decode::timestamp(
                    super::decode::value(&row, "withdrawn_at_seconds")?,
                    super::decode::value(&row, "withdrawn_at_nanoseconds")?,
                )
            })
            .transpose()?;
        // Every mutation/authority wait has completed before this clock read.
        let at = self.clock.now();
        validation::registration_time(at, command.not_before(), command.check())?;
        if let Some(previous) = previous {
            validation::after(at, previous)?;
        }
        let receipt = OwnerBindingReceipt {
            owner: actor,
            record: BindingRecord::registered(prepared.statement().clone()),
            check: command.check().clone(),
            trust: prepared.trust().clone(),
            registered_at: at,
            withdrawn_at: None,
        };
        validation::intent(&receipt, prepared, &command.check().signature)?;
        let event = crate::audit_postgres::append_transaction(
            &mut tx,
            &account.principal.email,
            "identity.owner_certificate_registered",
            &validation::resource(&receipt, false)?,
            at,
        )
        .map_err(|_| storage())?;
        insert(
            &mut tx,
            &receipt,
            &account.principal.email,
            event.event.sequence,
        )?;
        tx.commit().map_err(port)?;
        Ok(OwnerBindingCommit::Applied(receipt))
    }
}

fn insert(
    tx: &mut Transaction<'_>,
    receipt: &OwnerBindingReceipt,
    email: &str,
    sequence: u64,
) -> Result<(), ApplicationError> {
    let statement = receipt.record.registration();
    let material = statement.material();
    let owner = statement.owner();
    let check = &receipt.check;
    let summary = &check.certificate.summary;
    let bytes = statement.canonical_bytes();
    let sequence = i64::try_from(sequence).map_err(|_| inconsistent())?;
    tx.execute("INSERT INTO owner_certificate_registrations(
        binding_id,owner_id,account_revision,auth_generation,actor_email,deployment_id,trust_revision,
        root_fingerprint,leaf_fingerprint,statement,statement_digest,certificate_der,signature,
        certificate_subject,certificate_issuer,certificate_serial,certificate_not_before,certificate_not_after,
        checked_at,valid_from,valid_until,registered_at_seconds,registered_at_nanoseconds,audit_sequence)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)",
        &[&material.binding(), &receipt.owner.as_uuid(), &(owner.revision() as i64), &(owner.generation() as i64),
          &email, &material.deployment(), &i64::from(material.trust_revision()),
          &&material.root().as_bytes()[..], &&material.leaf().as_bytes()[..], &&bytes[..],
          &&check.statement_digest.as_bytes()[..], &check.certificate.der, &check.signature.as_bytes(),
          &summary.subject, &summary.issuer, &summary.serial_hex, &summary.not_before_unix, &summary.not_after_unix,
          &check.checked_at, &check.valid_from, &check.valid_until, &receipt.registered_at.unix_timestamp(),
          &(receipt.registered_at.nanosecond() as i32), &sequence]).map_err(port)?;
    Ok(())
}
