use domain::owner_certificates::Uuid;

use super::{
    evidence::same_registration, OwnerBindingCommit, OwnerBindingReceipt,
    OwnerCertificateError as Error, OwnerCertificateService, PreparedOwnerWithdrawal,
};
use crate::ApplicationError;

impl OwnerCertificateService {
    pub fn withdraw(
        &self,
        token: &str,
        binding: Uuid,
        expected_revision: u32,
    ) -> Result<OwnerBindingReceipt, ApplicationError> {
        let principal = self.owner(token)?;
        if expected_revision != 1 {
            return Err(Error::RevisionConflict.into());
        }
        if binding.is_nil() {
            return Err(Error::InvalidInput.into());
        }
        let context = self.ports.store.load_withdrawal(principal.id, binding)?;
        let account = self.account(&context.account, &principal)?;
        self.validate_receipt(&context.receipt, principal.id, binding)?;
        let captured = context
            .receipt
            .record
            .withdrawal()
            .map(|value| value.owner())
            .unwrap_or_else(|| context.receipt.record.registration().owner());
        if account.revision() < captured.revision() || account.generation() < captured.generation()
        {
            return Err(Error::AccountChanged.into());
        }
        if context.receipt.record.withdrawal().is_some() {
            self.reauthenticate(token, &principal)?;
            return Ok(context.receipt);
        }
        let record = context
            .receipt
            .record
            .withdraw(account, expected_revision)
            .map_err(|_| Error::Inconsistent)?;
        let not_before = self.ports.clock.now();
        if not_before < context.receipt.registered_at {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        let prepared = PreparedOwnerWithdrawal {
            account: context.account,
            original: context.receipt.clone(),
            record: record.clone(),
            not_before,
        };
        let (receipt, applied) = match self.ports.store.commit_withdrawal(prepared)? {
            OwnerBindingCommit::Applied(receipt) => (receipt, true),
            OwnerBindingCommit::Existing(receipt) => (receipt, false),
        };
        self.validate_receipt(&receipt, principal.id, binding)?;
        if !same_registration(&receipt, &context.receipt)
            || receipt.withdrawn_at.is_none()
            || (applied
                && (receipt.record != record
                    || receipt.withdrawn_at.is_some_and(|at| at < not_before)))
        {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}
