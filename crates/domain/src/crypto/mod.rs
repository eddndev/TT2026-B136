//! Value objects and outbound ports for the cryptographic operations of the
//! domain.
//!
//! Ports are traits implemented by adapters in outer layers. This module
//! holds the immutable values those operations exchange (content digests,
//! document identity, sealed payloads) and the port definitions themselves.

pub mod cipher;
pub mod digest;
pub mod document;
pub mod hasher;
pub mod keys;
pub mod password;
pub mod recovery;
pub mod totp;

pub use cipher::{document_aad, AuthenticatedCipher, SealedPayload};
pub use digest::Sha256Digest;
pub use document::{DocumentId, DocumentVersion};
pub use hasher::DocumentHasher;
pub use keys::{KeyManager, WrappedDek};
pub use password::{PasswordHasher, PasswordVerification};
pub use recovery::{
    RecoveryCodeGenerator, RecoveryCodeOutcome, RecoveryCodeSet, RECOVERY_CODE_COUNT,
};
pub use totp::{TotpEnrollment, TotpProvider, TotpVerification};
