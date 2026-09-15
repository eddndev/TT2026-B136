//! Concrete adapters that implement the domain's outbound ports.
//!
//! This crate wires the domain and use cases to real technology: hashing,
//! encryption, signing, timestamping, and persistence. Adapters are added
//! alongside their ports and tests. Backend errors are defined here and mapped
//! into [`application::ApplicationError`] at the call site.

pub mod archive;
pub mod audit;
mod audit_postgres;
pub mod cases;
pub mod certificates;
pub mod clock;
pub mod document_postgres;
pub mod documents;
pub mod encryption;
pub mod envelope;
pub mod error;
pub mod hashing;
pub mod identity;
pub mod legacy;
pub mod participant_postgres;
pub mod password;
mod postgres;
mod postgres_actor;
mod postgres_case_administration_inventory;
mod postgres_case_administration_schema;
mod postgres_case_status;
mod postgres_metadata_schema;
mod postgres_participant_schema;
mod postgres_version_schema;
pub mod recovery;
pub mod signing;
pub mod timestamp;
pub mod tools;
pub mod totp;

pub use archive::StoredZipWriter;
pub use audit::{FileAuditLog, InMemoryAuditLog};
pub use cases::PostgresCaseRepository;
pub use certificates::{OpensslCaAdapter, X509ChainValidator};
pub use clock::SystemClock;
pub use documents::FileDocumentRepository;
pub use encryption::RingAesGcmCipher;
pub use envelope::EnvelopeKeyManager;
pub use error::{CryptoError, TsaError};
pub use hashing::RingSha256Hasher;
pub use identity::{AesGcmSecretProtector, PostgresUserRepository, RedisSessionStore};
pub use participant_postgres::PostgresParticipantStore;
pub use password::Argon2idHasher;
pub use recovery::RandomRecoveryCodeGenerator;
pub use signing::{certificate_subject, RsaPkcs1Signer, RsaPkcs1Verifier};
pub use timestamp::{CincelTsaAdapter, LocalOpensslTsa, Rfc3161Verifier};
pub use tools::openssl_version;
pub use totp::{decode_base32_secret, TotpRsProvider};

pub use audit_postgres::PostgresAuditLog;
pub use document_postgres::PostgresCaseDocumentStore;
pub use legacy::{ImportReport, LegacyImport};
pub use postgres::initialize_database;
