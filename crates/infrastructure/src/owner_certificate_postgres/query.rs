use application::{identity::owner_certificates::*, ApplicationError};
use domain::identity::UserId;
use postgres::GenericClient;
use uuid::Uuid;

use super::{account, decode, inconsistent, port, storage, PostgresOwnerCertificateStore};

impl PostgresOwnerCertificateStore {
    pub(super) fn registration_context(
        &self,
        actor: UserId,
    ) -> Result<OwnerRegistrationContext, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        let account = account::owner(&mut tx, actor)?;
        let trust = crate::credential_trust_postgres::current(&mut tx).map_err(|_| storage())?;
        tx.commit().map_err(port)?;
        Ok(OwnerRegistrationContext { account, trust })
    }

    pub(super) fn receipt(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        account::owner(&mut tx, actor)?;
        let receipt = load(&mut tx, binding, Some(actor))?;
        tx.commit().map_err(port)?;
        Ok(receipt)
    }

    pub(super) fn current_receipt(
        &self,
        actor: UserId,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        account::owner(&mut tx, actor)?;
        let rows = tx
            .query(
                "SELECT r.binding_id FROM owner_certificate_registrations r
        WHERE r.owner_id=$1 AND NOT EXISTS(
            SELECT 1 FROM owner_certificate_withdrawals w WHERE w.binding_id=r.binding_id)
        LIMIT 2",
                &[&actor.as_uuid()],
            )
            .map_err(port)?;
        let receipt = match rows.as_slice() {
            [] => None,
            [row] => {
                let binding = decode::value(row, "binding_id")?;
                let receipt = load(&mut tx, binding, Some(actor))?.ok_or_else(inconsistent)?;
                if receipt.record.withdrawal().is_some() {
                    return Err(inconsistent());
                }
                Some(receipt)
            }
            _ => return Err(inconsistent()),
        };
        tx.commit().map_err(port)?;
        Ok(receipt)
    }

    pub(super) fn withdrawal_context(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<OwnerWithdrawalContext, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client).map_err(|_| storage())?;
        let account = account::owner(&mut tx, actor)?;
        let receipt =
            load(&mut tx, binding, Some(actor))?.ok_or(OwnerCertificateError::NotFound)?;
        tx.commit().map_err(port)?;
        Ok(OwnerWithdrawalContext { account, receipt })
    }
}

pub(super) fn load<C: GenericClient>(
    client: &mut C,
    binding: Uuid,
    owner: Option<UserId>,
) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
    // Immutable rows need no UPDATE grant: the shared audit lock excludes writers.
    let row = client
        .query_opt(
            "SELECT * FROM owner_certificate_registrations
        WHERE binding_id=$1 AND ($2::uuid IS NULL OR owner_id=$2)",
            &[&binding, &owner.map(|id| id.as_uuid())],
        )
        .map_err(port)?;
    let Some(row) = row else { return Ok(None) };
    let mut receipt = decode::registration(client, &row)?;
    if let Some(row) = client
        .query_opt(
            "SELECT * FROM owner_certificate_withdrawals
        WHERE binding_id=$1",
            &[&binding],
        )
        .map_err(port)?
    {
        decode::withdrawal(&mut receipt, &row)?;
    }
    Ok(Some(receipt))
}
