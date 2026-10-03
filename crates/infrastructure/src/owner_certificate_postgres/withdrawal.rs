use application::{identity::owner_certificates::*, ApplicationError};

use super::{
    account, inconsistent, port, query, storage, validation, PostgresOwnerCertificateStore,
};

impl PostgresOwnerCertificateStore {
    pub(super) fn withdraw(
        &self,
        command: PreparedOwnerWithdrawal,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        let actor = command.account().principal.id;
        let binding = command
            .original()
            .record
            .registration()
            .material()
            .binding();
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        let account = account::owner(&mut tx, actor)?;
        let mut receipt =
            query::load(&mut tx, binding, Some(actor))?.ok_or(OwnerCertificateError::NotFound)?;
        if !validation::same_registration(&receipt, command.original()) {
            return Err(inconsistent());
        }
        // Preserve the original withdrawal even if this command captured older counters.
        if receipt.record.withdrawal().is_some() {
            tx.commit().map_err(port)?;
            return Ok(OwnerBindingCommit::Existing(receipt));
        }
        if account != *command.account() {
            return Err(OwnerCertificateError::AccountChanged.into());
        }
        let captured = account::captured(
            actor,
            account.revision as i64,
            account.auth_generation as i64,
        )?;
        let record = receipt
            .record
            .withdraw(captured, 1)
            .map_err(|_| inconsistent())?;
        if &record != command.record() {
            return Err(inconsistent());
        }
        let at = self.clock.now();
        validation::after(at, command.not_before())?;
        validation::after(at, receipt.registered_at)?;
        receipt.record = record;
        receipt.withdrawn_at = Some(at);
        validation::receipt(&receipt)?;
        let event = crate::audit_postgres::append_transaction(
            &mut tx,
            &account.principal.email,
            "identity.owner_certificate_withdrawn",
            &validation::resource(&receipt, true)?,
            at,
        )
        .map_err(|_| storage())?;
        let bytes = receipt
            .record
            .withdrawal()
            .ok_or_else(inconsistent)?
            .canonical_bytes();
        let digest = validation::digest(&bytes);
        let sequence = i64::try_from(event.event.sequence).map_err(|_| inconsistent())?;
        tx.execute("INSERT INTO owner_certificate_withdrawals(binding_id,account_revision,auth_generation,
            actor_email,statement,statement_digest,withdrawn_at_seconds,withdrawn_at_nanoseconds,audit_sequence)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            &[&binding, &(account.revision as i64), &(account.auth_generation as i64),
              &account.principal.email, &&bytes[..], &&digest.as_bytes()[..], &at.unix_timestamp(),
              &(at.nanosecond() as i32), &sequence]).map_err(port)?;
        tx.commit().map_err(port)?;
        Ok(OwnerBindingCommit::Applied(receipt))
    }
}
