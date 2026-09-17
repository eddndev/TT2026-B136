//! Errors surfaced by use cases.

use domain::DomainError;
use thiserror::Error;

/// Failure of a use case.
///
/// A use case either violates a domain invariant (wrapped from
/// [`DomainError`]) or fails while talking to an outbound port. Port failures
/// are reported as a message so this crate stays free of adapter details.
#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    DeadlineInput(#[from] crate::deadline_inputs::DeadlineInputError),

    #[error(transparent)]
    ProceduralFact(#[from] crate::procedural_facts::ProceduralFactError),

    #[error(transparent)]
    JudicialCalendar(#[from] crate::judicial_calendars::JudicialCalendarError),

    #[error(transparent)]
    HearingResult(#[from] crate::hearing_results::HearingResultError),

    #[error(transparent)]
    Hearing(#[from] crate::hearings::HearingError),

    #[error("represented subject not found")]
    SubjectNotFound,

    #[error("represented subject changed")]
    SubjectRevisionConflict,

    #[error("represented subject revision counter is exhausted")]
    SubjectRevisionExhausted,

    #[error("typed participant profile is required")]
    ParticipantProfileRequired,

    #[error("represented subject already has this role")]
    ParticipantRoleConflict,

    #[error("represented identity of a typed participant cannot be replaced")]
    ParticipantSubjectChangeForbidden,

    #[error("explicit represented identity review is required")]
    ParticipantIdentityReviewRequired,

    #[error("represented identity review changed")]
    ParticipantIdentityReviewConflict,

    #[error("identity review exceeds the candidate limit")]
    ParticipantCandidateLimit,

    #[error("participant support changed; validate it again")]
    ParticipantSupportChanged,

    #[error("participant credential and declaration signature are required")]
    ParticipantCredentialRequired,

    #[error("participant credential is not applicable")]
    ParticipantCredentialUnexpected,

    #[error("participant credential not found")]
    ParticipantCredentialNotFound,

    #[error("participant credential trust is not published")]
    CredentialTrustUnavailable,

    #[error("participant credential trust changed")]
    CredentialTrustChanged,

    #[error("participant credential trust revision changed")]
    CredentialTrustRevisionConflict,

    #[error("participant credential trust revision counter is exhausted")]
    CredentialTrustRevisionExhausted,
    #[error("participant credential rejected: {0}")]
    ParticipantCredentialRejected(#[from] domain::crypto::CredentialFailure),

    /// The command expected another current stage revision.
    #[error("case stage changed; refresh before recording")]
    CaseStageConflict,

    /// Stage revision counters never wrap.
    #[error("case stage revision counter is exhausted")]
    CaseStageRevisionExhausted,

    /// An ordinary transition requires a registered starting stage.
    #[error("register the current case stage before advancing")]
    CaseStageRequired,

    /// The requested target does not follow the current registered stage.
    #[error("case stage transition is not permitted")]
    CaseStageTransitionRejected,

    /// A stage mutation requires the current complete penal profile.
    #[error("complete the penal case profile before recording its stage")]
    CaseStageProfileIncomplete,

    /// The selected digest disagrees with the authorized exact version.
    #[error("selected stage support digest does not match the stored version")]
    StageSupportDigestMismatch,

    /// Evidence changed after the support validation was prepared.
    #[error("stage support changed; validate it again before recording")]
    StageSupportChanged,

    /// A selected document exceeds the bounded support-admission policy.
    #[error("document exceeds the stage support size limit")]
    StageSupportTooLarge,

    /// Verified content does not satisfy the supported document-format profile.
    #[error("document format is not admitted as a stage support")]
    StageSupportFormatRejected,

    /// Format parsing exhausted a configured resource allowance.
    #[error("document validation exceeded its resource limit")]
    StageSupportValidationLimit,

    /// Persisted stage values or historical provenance violate invariants.
    #[error("stored case stage is inconsistent: {0}")]
    StoredCaseStageInconsistent(String),

    /// The caller expected a different current administrative head.
    #[error("case changed; refresh before replacing")]
    CaseRevisionConflict,

    /// Administrative revisions cannot wrap after their maximum counter.
    #[error("case revision counter is exhausted")]
    CaseRevisionExhausted,

    /// A currently closed case disallows this authorized mutation.
    #[error("case is administratively closed")]
    CaseClosed,

    /// A current profile already claims one of the supplied identifiers.
    #[error("case identifier is already registered")]
    CaseIdentifierConflict,

    /// A completed profile cannot be replaced with an absent profile.
    #[error("completed case profile cannot be removed")]
    CaseProfileRequired,

    /// Stored administration or initial-stage provenance is inconsistent.
    #[error("stored case administration is inconsistent: {0}")]
    StoredCaseAdministrationInconsistent(String),

    /// A participant is absent or outside the requested visible case.
    #[error("participant not found")]
    ParticipantNotFound,

    /// A participant replacement expected another current revision.
    #[error("participant changed; refresh before replacing")]
    ParticipantRevisionConflict,

    /// A participant cannot append another revision.
    #[error("participant revision counter is exhausted")]
    ParticipantRevisionExhausted,

    /// Stored participant values or provenance violate their invariants.
    #[error("stored participant is inconsistent: {0}")]
    StoredParticipantInconsistent(String),

    /// Bootstrap was requested after the first user had already been stored.
    #[error("initial owner has already been created")]
    BootstrapClosed,

    /// A user email is already registered.
    #[error("user already exists")]
    UserAlreadyExists,

    /// A requested user does not exist.
    #[error("user not found")]
    UserNotFound,

    /// A case is absent or not visible to the authenticated user.
    #[error("case not found")]
    CaseNotFound,

    /// A password or account identifier was rejected.
    #[error("invalid credentials")]
    InvalidCredentials,

    /// The login window is temporarily locked after repeated failures.
    #[error("account temporarily locked")]
    AccountLocked,

    /// A second-factor or recovery code was rejected.
    #[error("second factor rejected")]
    MfaRejected,

    /// A bearer token is absent, expired, revoked, or otherwise invalid.
    #[error("invalid session")]
    InvalidSession,

    /// The authenticated role does not grant the requested action.
    #[error("permission denied")]
    PermissionDenied,

    /// An optimistic update lost a race with another request.
    #[error("concurrent modification")]
    ConcurrentModification,

    /// A domain invariant was violated.
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// An outbound port reported a failure.
    #[error("port failure: {0}")]
    Port(String),

    /// Bytes presented as a vault file did not match the vault file format
    /// documented in the `vault` module.
    #[error("malformed vault file: {0}")]
    MalformedVaultFile(String),

    /// A freshly issued certificate failed the validation that runs
    /// before issuance is reported as successful; the message carries the
    /// validation outcome.
    #[error("issued certificate failed post-issuance validation: {0}")]
    IssuedCertificateInvalid(String),

    /// A requested document identity has no stored record.
    #[error("document not found: {0}")]
    DocumentNotFound(String),

    /// A repository already contains the identity being inserted.
    #[error("document already exists: {0}")]
    DocumentAlreadyExists(String),

    /// An append expected a different current version.
    #[error("document version changed; refresh before appending")]
    DocumentVersionConflict,

    /// A document has multiple versions and the operation did not select one.
    #[error("document has multiple versions; select an explicit version")]
    DocumentVersionRequired,

    /// A document cannot increment its maximum version number.
    #[error("document version counter is exhausted")]
    DocumentVersionExhausted,

    /// A classification replacement expected another current revision.
    #[error("document metadata changed; refresh before replacing")]
    DocumentMetadataConflict,

    /// A classification revision cannot be incremented further.
    #[error("document metadata revision counter is exhausted")]
    DocumentMetadataRevisionExhausted,

    /// Persisted organizational values violate canonical form or digest.
    #[error("stored document metadata is inconsistent: {0}")]
    StoredDocumentMetadataInconsistent(String),

    /// A document already has immutable signature and timestamp evidence.
    #[error("document is already sealed: {0}")]
    DocumentAlreadySealed(String),

    /// An operation requires evidence that has not been created yet.
    #[error("document is not sealed: {0}")]
    DocumentNotSealed(String),

    /// Stored metadata contradicts the encrypted document content.
    #[error("stored document is inconsistent: {0}")]
    StoredDocumentInconsistent(String),

    /// Runtime material cannot satisfy the workflow's fixed contracts.
    #[error("invalid application configuration: {0}")]
    InvalidConfiguration(String),

    /// Freshly produced evidence failed its immediate integrity check.
    #[error("timestamp evidence was rejected: {0}")]
    TimestampEvidenceRejected(String),

    /// A caller supplied an unusable application-level value.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
