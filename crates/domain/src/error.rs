//! Errors raised when domain invariants are violated.

use thiserror::Error;

/// Failure to build or operate on a domain value.
///
/// Variants describe violations of value-object invariants. They carry enough
/// detail for a caller to report the cause without inspecting internals.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid hearing result {0}")]
    InvalidHearingResultValue(&'static str),

    #[error("hearing result revision must be positive")]
    InvalidHearingResultRevision,

    #[error("invalid hearing {0}")]
    InvalidHearingValue(&'static str),

    #[error("hearing revision must be positive")]
    InvalidHearingRevision,

    #[error("invalid typed participant {0}")]
    InvalidTypedParticipantValue(&'static str),
    #[error("subject revision must be positive")]
    InvalidSubjectRevision,
    #[error("subject revision counter is exhausted")]
    SubjectRevisionExhausted,
    #[error("same participant support has inconsistent digests")]
    ParticipantSupportDigestMismatch,
    #[error("participant role and represented subject kind disagree")]
    ParticipantSubjectKindMismatch,

    #[error("invalid case stage")]
    InvalidCaseStage,

    #[error("declared stage time has an invalid year or offset")]
    InvalidDeclaredStageTime,

    #[error("stage note must contain 1 to 1000 characters without unsupported controls")]
    InvalidStageNote,

    #[error("stage court must contain 1 to 200 characters without controls")]
    InvalidStageCourt,

    #[error("stage receipt reference must contain 1 to 200 characters without controls")]
    InvalidStageReceiptReference,

    #[error("opening order issuance is incompatible with the declared receipt time")]
    InvalidStageActOrder,

    #[error("one exact document support cannot have two digests")]
    ConflictingStageSupport,

    #[error("declared stage act occurs after the captured recording time")]
    StageActInFuture,

    /// A manual penal profile violates bounded field invariants.
    #[error("invalid penal case {field}: {reason}")]
    InvalidPenalCaseProfile {
        field: &'static str,
        reason: &'static str,
    },

    /// Administrative history never stores a revision zero.
    #[error("case revision must be 1 or greater")]
    InvalidCaseRevision,

    /// Stage history uses its own positive revision counter.
    #[error("case stage revision must be 1 or greater")]
    InvalidCaseStageRevision,

    /// Administrative status is active or closed; rejected input is not retained.
    #[error("invalid case administrative status")]
    InvalidCaseAdministrativeStatus,

    /// Case-local participant fields violate their bounded text invariants.
    #[error("invalid participant {field}: {reason}")]
    InvalidParticipantValues {
        field: &'static str,
        reason: &'static str,
    },

    /// Participant histories start at one and have no implicit zero snapshot.
    #[error("participant revision must be 1 or greater")]
    InvalidParticipantRevision,

    /// The organizational directory state has no such value.
    #[error("unknown directory status: {0}")]
    InvalidDirectoryStatus(String),

    /// Organizational fields are malformed or exceed their bounded size.
    #[error("invalid document metadata {field}: {reason}")]
    InvalidDocumentMetadata {
        field: &'static str,
        reason: &'static str,
    },

    /// Case metadata is missing, contains controls, or exceeds its size limit.
    #[error("invalid case {field}: {reason}")]
    InvalidCaseMetadata {
        field: &'static str,
        reason: &'static str,
    },

    /// A persisted role name is not part of the authorization vocabulary.
    #[error("unknown role: {0}")]
    InvalidRole(String),

    /// A digest was built from a byte slice of the wrong length.
    #[error("digest must be {expected} bytes, got {actual}")]
    InvalidDigestLength { expected: usize, actual: usize },

    /// A hex string could not be decoded into the expected byte length.
    #[error("invalid hex encoding for a {expected}-byte value")]
    InvalidHexEncoding { expected: usize },

    /// A document version was outside the allowed range (versions start at 1).
    #[error("document version must be 1 or greater")]
    InvalidDocumentVersion,

    /// A document already uses the largest representable version counter.
    #[error("document version counter is exhausted")]
    DocumentVersionExhausted,

    /// Reading from an input stream failed while digesting its content.
    #[error("failed to read input stream: {message}")]
    StreamRead { message: String },

    /// An authenticated decryption was rejected. Deliberately opaque: it
    /// does not reveal whether the key, the tag, or the data was wrong.
    #[error("authenticated decryption failed")]
    AuthenticationFailed,

    /// Key material was unusable: wrong length or rejected by the backend.
    #[error("invalid key material: {0}")]
    InvalidKeyMaterial(String),

    /// A sealed payload was too short to hold a nonce and a tag.
    #[error("sealed payload must be at least {min} bytes, got {actual}")]
    MalformedSealedPayload { min: usize, actual: usize },

    /// The cryptographic backend failed for a reason other than a rejected
    /// authentication check.
    #[error("cryptographic backend failure: {0}")]
    CryptoBackendFailure(String),

    /// A stored password hash could not be parsed as a PHC string.
    #[error("stored password hash is not a valid phc string")]
    MalformedPasswordHash,

    /// The hashing backend failed while deriving a credential hash.
    #[error("password hashing failed: {0}")]
    PasswordHashingFailed(String),

    /// A one-time-password secret was shorter than the backend allows.
    #[error("totp secret must be at least {minimum} bytes, got {actual}")]
    TotpSecretTooShort { minimum: usize, actual: usize },

    /// The one-time-password backend failed.
    #[error("totp backend failure: {0}")]
    TotpBackendFailed(String),

    /// The operating system's random generator failed.
    #[error("random generation failed: {0}")]
    RandomnessFailed(String),

    /// A recovery code set was built from the wrong number of hashes.
    #[error("recovery code set must hold {expected} codes, got {actual}")]
    InvalidRecoveryCodeCount { expected: usize, actual: usize },

    /// An audit event timestamp could not be rendered as an RFC 3339 string.
    #[error("audit timestamp cannot be rendered as rfc 3339: {0}")]
    TimestampNotRepresentable(String),

    /// An audit event field did not fit the 4-byte length prefix of the
    /// canonical encoding.
    #[error("audit field length exceeds {max} bytes")]
    AuditFieldTooLong { max: usize },

    /// Reading or writing audit log storage failed, or a stored entry could
    /// not be decoded.
    #[error("audit log storage failure: {0}")]
    AuditStorageFailure(String),

    /// Certificate or issuer bytes could not be parsed as X.509 data.
    #[error("certificate cannot be parsed: {0}")]
    MalformedCertificate(String),

    /// Revocation-list bytes could not be parsed as an X.509 CRL.
    #[error("certificate revocation list cannot be parsed: {0}")]
    MalformedCrl(String),

    /// The revocation list's next scheduled update is already in the past
    /// at the evaluation time, so its revocation data cannot be relied on.
    #[error(
        "certificate revocation list is stale: next update was due at unix time {next_update_unix}"
    )]
    StaleCrl { next_update_unix: i64 },

    /// The revocation list is not signed by the presented issuer.
    #[error("certificate revocation list is not signed by the issuer")]
    UntrustedCrl,

    /// A certificate-authority operation failed.
    #[error("certificate authority operation failed: {0}")]
    CertificateAuthorityFailure(String),

    /// A signature value was built from an empty byte sequence.
    #[error("signature must not be empty")]
    EmptySignature,

    /// A timestamp authority request failed: the authority was
    /// unreachable, rejected the request, or returned an unusable
    /// response.
    #[error("timestamp authority failure: {0}")]
    TimestampAuthorityFailure(String),

    /// An archive entry name fell outside the safe set that keeps names
    /// usable in file systems, shell command lines, and the verification
    /// instructions shipped inside the archive.
    #[error(
        "archive entry name is not usable: {name:?} (ascii letters, digits, \
         dot, dash, and underscore only, starting with a letter or digit)"
    )]
    InvalidArchiveEntryName { name: String },

    /// The archive writer could not represent the entries in its format.
    #[error("archive writing failed: {0}")]
    ArchiveWriteFailure(String),
}
