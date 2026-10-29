use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialCheck, Sha256Digest},
    document_signing::{DocumentConsent, SignerAccount},
};

use super::SigningBindingReceipt;
use crate::{
    credential_trust::CredentialTrustSnapshot,
    documents::{DocumentRecord, SealedEvidence, SigningManifest},
    identity::Principal,
};

/// An exact public document consent together with its durably accepted evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningReceipt {
    pub consent: DocumentConsent,
    pub evidence: SealedEvidence,
    pub accepted_at: OffsetDateTime,
    /// The sealed evidence manifest; `None` only for receipts accepted
    /// before the manifest table existed.
    pub manifest: Option<SigningManifest>,
}

/// Distinguishes a new atomic seal from recovery of a concurrently accepted seal.
pub enum SigningCommit {
    Applied(SigningReceipt),
    Existing(SigningReceipt),
}

/// A per-case authorized immutable version and optional current public authority.
pub struct SigningVerificationContext {
    pub signing: Option<SigningReceipt>,
    pub record: DocumentRecord,
    pub trust: Option<CredentialTrustSnapshot>,
    pub binding_withdrawn_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentSigningStatus {
    NotIndividual,
    TrustUnavailable,
    Valid,
    Rejected(domain::crypto::CredentialFailure),
}

pub struct SigningVerification {
    pub historical: crate::verification::VerificationReport,
    pub certificate_current: CurrentSigningStatus,
    pub checked_at: OffsetDateTime,
    pub binding_withdrawn_at: Option<OffsetDateTime>,
    pub signing: Option<SigningReceipt>,
}

/// Admitted PDF identity and the public context reviewed before external consent.
pub struct PreparedDocumentSigning {
    pub(super) principal: Principal,
    pub(super) account: SignerAccount,
    pub(super) trust: CredentialTrustSnapshot,
    pub(super) binding: SigningBindingReceipt,
    pub(super) record: DocumentRecord,
    pub(super) consent: DocumentConsent,
}

impl PreparedDocumentSigning {
    pub const fn consent(&self) -> &DocumentConsent {
        &self.consent
    }
    pub fn name(&self) -> &str {
        &self.record.name
    }
    pub const fn certificate_fingerprint(&self) -> Sha256Digest {
        self.consent.material().leaf()
    }
}

/// Only the application service constructs an authorized, doubly verified seal.
/// Stores must still recheck mutable authority, exact content and time under locks.
pub struct VerifiedDocumentSigning {
    pub(super) prepared: PreparedDocumentSigning,
    pub(super) check: CredentialCheck,
    pub(super) record: DocumentRecord,
    pub(super) not_before: OffsetDateTime,
    pub(super) manifest: SigningManifest,
}

impl VerifiedDocumentSigning {
    pub const fn principal(&self) -> &Principal {
        &self.prepared.principal
    }
    pub const fn account(&self) -> &SignerAccount {
        &self.prepared.account
    }
    pub const fn trust(&self) -> &CredentialTrustSnapshot {
        &self.prepared.trust
    }
    pub const fn binding(&self) -> &SigningBindingReceipt {
        &self.prepared.binding
    }
    pub const fn record(&self) -> &DocumentRecord {
        &self.record
    }
    pub const fn consent(&self) -> &DocumentConsent {
        &self.prepared.consent
    }
    pub const fn check(&self) -> &CredentialCheck {
        &self.check
    }
    pub const fn not_before(&self) -> OffsetDateTime {
        self.not_before
    }
    pub const fn manifest(&self) -> &SigningManifest {
        &self.manifest
    }
}
