//! Concrete adapters that implement the domain's outbound ports.
//!
//! This crate wires the domain and use cases to real technology: hashing,
//! encryption, signing, timestamping, and persistence. Adapters are added
//! alongside their ports and tests. Backend errors are defined here and mapped
//! into [`application::ApplicationError`] at the call site.

pub mod encryption;
pub mod envelope;
pub mod error;
pub mod hashing;

pub use encryption::RingAesGcmCipher;
pub use envelope::EnvelopeKeyManager;
pub use error::{CryptoError, TsaError};
pub use hashing::RingSha256Hasher;
