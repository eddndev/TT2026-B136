//! Concrete adapters that implement the domain's outbound ports.
//!
//! This crate wires the domain and use cases to real technology: hashing,
//! encryption, signing, timestamping, and persistence. Adapters are added
//! alongside their ports and tests. Backend errors are defined here and mapped
//! into [`application::ApplicationError`] at the call site.

pub mod archive;
pub mod audit;
mod audit_postgres;
pub mod case_stages;
pub mod cases;
pub mod certificates;
pub mod clock;
pub mod credential_trust_postgres;
#[doc(hidden)]
pub mod deadline_input_history;
mod deadline_input_postgres;
mod deadline_source_event_schema;
pub use deadline_input_postgres::PostgresDeadlineInputStore;
pub mod document_postgres;
pub mod documents;
pub mod encryption;
pub mod envelope;
pub mod error;
pub mod hashing;
pub mod hearing_codec;
mod hearing_postgres;
pub mod hearing_result_codec;
pub use hearing_postgres::PostgresHearingStore;
mod hearing_result_postgres;
mod hearing_result_schema;
mod hearing_schema;
pub use hearing_result_postgres::PostgresHearingResultStore;
mod judicial_calendar_postgres;
mod judicial_calendar_schema;
pub use judicial_calendar_postgres::PostgresJudicialCalendarStore;
pub mod identity;
pub mod legacy;
pub mod participant_postgres;
pub mod password;
pub mod procedural_fact_codec;
pub mod procedural_fact_postgres;
mod procedural_fact_schema;
pub use procedural_fact_postgres::PostgresProceduralFactStore;
mod postgres;
mod postgres_actor;
mod postgres_case_administration_inventory;
mod postgres_case_administration_schema;
mod postgres_case_stages_inventory;
mod postgres_case_stages_schema;
mod postgres_case_status;
mod postgres_metadata_schema;
mod postgres_participant_schema;
mod postgres_port;
mod postgres_version_schema;
pub mod recovery;
pub mod signing;
pub mod timestamp;
pub mod tools;
pub mod totp;
pub mod typed_participant_codec;
pub(crate) mod typed_participant_schema;

pub use archive::StoredZipWriter;
pub use audit::{FileAuditLog, InMemoryAuditLog};
pub use case_stages::PostgresCaseStageStore;
pub use cases::PostgresCaseRepository;
pub use certificates::{OpensslCaAdapter, X509ChainValidator};
pub use clock::SystemClock;
pub use credential_trust_postgres::PostgresCredentialTrustStore;
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
pub mod document_formats;

pub(crate) mod typed_participant_postgres;
pub use typed_participant_postgres::PostgresTypedParticipantStore;

mod deadline_profile_postgres;
pub use deadline_profile_postgres::PostgresDeadlineProfileStore;

mod deadline_profile_schema;

mod deadline_postgres;
pub use deadline_postgres::PostgresDeadlineStore;
mod deadline_schema;

mod deadline_dispatch_postgres;
mod deadline_dispatch_schema;
pub use deadline_dispatch_postgres::PostgresDeadlineDispatchStore;

mod deadline_worker_provenance;

mod deadline_worker_postgres;
mod deadline_worker_schema;
pub use deadline_worker_postgres::PostgresDeadlineWorkerStore;
