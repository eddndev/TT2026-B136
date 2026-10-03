use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialCertificate, CredentialCheck},
    owner_certificates::{BindingRecord, BindingStatement},
};

use super::{OwnerBindingAccount, OwnerBindingReceipt};
use crate::credential_trust::CredentialTrustSnapshot;

/// Opaque public signing intent, not a bearer capability or authentication proof.
#[derive(Clone)]
pub struct PreparedOwnerRegistration {
    pub(super) account: OwnerBindingAccount,
    pub(super) statement: BindingStatement,
    pub(super) certificate: CredentialCertificate,
    pub(super) trust: CredentialTrustSnapshot,
}

impl PreparedOwnerRegistration {
    pub fn account(&self) -> &OwnerBindingAccount {
        &self.account
    }
    pub fn statement(&self) -> &BindingStatement {
        &self.statement
    }
    pub fn certificate(&self) -> &CredentialCertificate {
        &self.certificate
    }
    pub fn trust(&self) -> &CredentialTrustSnapshot {
        &self.trust
    }
}

/// Verification is provisional until the store rechecks all mutable facts.
pub struct VerifiedOwnerRegistration {
    pub(super) registration: PreparedOwnerRegistration,
    pub(super) check: CredentialCheck,
    pub(super) not_before: OffsetDateTime,
}

impl VerifiedOwnerRegistration {
    pub fn registration(&self) -> &PreparedOwnerRegistration {
        &self.registration
    }
    pub fn check(&self) -> &CredentialCheck {
        &self.check
    }
    pub fn not_before(&self) -> OffsetDateTime {
        self.not_before
    }
}

/// Only the application can prepare the checked live-to-terminal transition.
pub struct PreparedOwnerWithdrawal {
    pub(super) account: OwnerBindingAccount,
    pub(super) original: OwnerBindingReceipt,
    pub(super) record: BindingRecord,
    pub(super) not_before: OffsetDateTime,
}

impl PreparedOwnerWithdrawal {
    pub fn account(&self) -> &OwnerBindingAccount {
        &self.account
    }
    pub fn original(&self) -> &OwnerBindingReceipt {
        &self.original
    }
    pub fn record(&self) -> &BindingRecord {
        &self.record
    }
    pub fn not_before(&self) -> OffsetDateTime {
        self.not_before
    }
}
