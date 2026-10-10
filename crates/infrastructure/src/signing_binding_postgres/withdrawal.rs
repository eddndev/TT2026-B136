use super::{account, inconsistent, load, port, resource, PostgresSigningBindingStore};
use application::{
    document_signing::{SigningBindingReceipt, SigningWorkflowError as Error},
    identity::Principal,
    ApplicationError,
};
use domain::{clock::OffsetDateTime, document_signing::SignerAccount};
use uuid::Uuid;

impl PostgresSigningBindingStore {
    pub(super) fn withdraw_binding(
        &self,
        principal: &Principal,
        binding: Uuid,
        expected: SignerAccount,
        not_before: OffsetDateTime,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let context = account(&mut tx, principal.id)?;
        if context.principal != *principal {
            return Err(Error::AccountChanged.into());
        }
        let mut receipt = load(&mut tx, binding)?
            .filter(|receipt| receipt.statement.account().id() == principal.id)
            .ok_or(Error::NotFound)?;
        if receipt.withdrawn_at.is_some() {
            tx.commit().map_err(port)?;
            return Ok(receipt);
        }
        if context.account != expected
            || expected.revision() < receipt.statement.account().revision()
            || expected.generation() < receipt.statement.account().generation()
        {
            return Err(Error::AccountChanged.into());
        }
        let at = self.clock.now();
        if at < not_before || at < receipt.registered_at {
            return Err(inconsistent());
        }
        let event = crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "document.signer_withdrawn",
            &resource(&receipt, true),
            at,
        )?;
        let sequence = i64::try_from(event.event.sequence).map_err(|_| inconsistent())?;
        tx.execute("INSERT INTO document_signing_withdrawals(binding_id,account_revision,auth_generation,actor_email,
            withdrawn_at_seconds,withdrawn_at_nanoseconds,audit_sequence) VALUES($1,$2,$3,$4,$5,$6,$7)",
            &[&binding, &(expected.revision() as i64), &(expected.generation() as i64), &principal.email,
                &at.unix_timestamp(), &(at.nanosecond() as i32), &sequence]).map_err(port)?;
        tx.commit().map_err(port)?;
        receipt.withdrawn_at = Some(at);
        Ok(receipt)
    }
}
