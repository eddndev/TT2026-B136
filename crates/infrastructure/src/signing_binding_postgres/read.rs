use super::{account, decode, inconsistent, port, PostgresSigningBindingStore};
use application::{
    document_signing::{SigningAccountContext, SigningBindingReceipt},
    ApplicationError,
};
use domain::identity::UserId;
use postgres::GenericClient;
use uuid::Uuid;

impl PostgresSigningBindingStore {
    pub(super) fn context(&self, actor: UserId) -> Result<SigningAccountContext, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let mut context = account(&mut tx, actor)?;
        context.trust = crate::credential_trust_postgres::current(&mut tx)?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &context.principal.email,
            "document.signer_context_read",
            &format!("document-signer:account:{actor}"),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(context)
    }
    pub(super) fn find_receipt(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let context = account(&mut tx, actor)?;
        let receipt =
            load(&mut tx, binding)?.filter(|receipt| receipt.statement.account().id() == actor);
        crate::audit_postgres::append_transaction(
            &mut tx,
            &context.principal.email,
            "document.signer_binding_read",
            &format!("document-signer:{binding}:signer:{actor}"),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(receipt)
    }
    pub(super) fn current_receipt(
        &self,
        actor: UserId,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let context = account(&mut tx, actor)?;
        let rows = tx.query("SELECT binding_id FROM document_signing_bindings r
            WHERE signer_id=$1 AND NOT EXISTS(SELECT 1 FROM document_signing_withdrawals w WHERE w.binding_id=r.binding_id)
            LIMIT 2", &[&actor.as_uuid()]).map_err(port)?;
        let receipt = match rows.as_slice() {
            [] => None,
            [row] => {
                Some(load(&mut tx, decode::value(row, "binding_id")?)?.ok_or_else(inconsistent)?)
            }
            _ => return Err(inconsistent()),
        };
        crate::audit_postgres::append_transaction(
            &mut tx,
            &context.principal.email,
            "document.signer_current_read",
            &format!("document-signer:account:{actor}"),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(receipt)
    }
}

/// Loads captured history inside the caller's audited and authorized transaction.
pub(crate) fn load<C: GenericClient>(
    client: &mut C,
    binding: Uuid,
) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
    let Some(row) = client
        .query_opt(
            "SELECT * FROM document_signing_bindings WHERE binding_id=$1",
            &[&binding],
        )
        .map_err(port)?
    else {
        return Ok(None);
    };
    let mut receipt = decode::registration(client, &row)?;
    if let Some(row) = client
        .query_opt(
            "SELECT * FROM document_signing_withdrawals WHERE binding_id=$1",
            &[&binding],
        )
        .map_err(port)?
    {
        decode::withdrawal(&mut receipt, &row)?;
    }
    Ok(Some(receipt))
}
