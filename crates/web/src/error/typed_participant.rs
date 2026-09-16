use super::ApiError;
use application::ApplicationError as A;
use axum::http::StatusCode;
use domain::{crypto::CredentialFailure as C, DomainError as D};

pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::SubjectNotFound => (404, "subject_not_found"),
        A::SubjectRevisionConflict => (409, "subject_revision_conflict"),
        A::SubjectRevisionExhausted => (409, "subject_revision_exhausted"),
        A::ParticipantProfileRequired => (409, "participant_profile_required"),
        A::ParticipantRoleConflict => (409, "participant_role_conflict"),
        A::ParticipantSubjectChangeForbidden => (409, "participant_subject_change_forbidden"),
        A::ParticipantIdentityReviewRequired => (409, "participant_identity_review_required"),
        A::ParticipantIdentityReviewConflict => (409, "participant_identity_review_conflict"),
        A::ParticipantCandidateLimit => (422, "participant_candidate_limit"),
        A::ParticipantSupportChanged => (409, "participant_support_changed"),
        A::ParticipantCredentialRequired => (422, "participant_credential_required"),
        A::ParticipantCredentialUnexpected => (422, "participant_credential_unexpected"),
        A::ParticipantCredentialNotFound => (404, "participant_credential_not_found"),
        A::CredentialTrustUnavailable => (409, "credential_trust_unavailable"),
        A::CredentialTrustChanged => (409, "credential_trust_changed"),
        A::CredentialTrustRevisionConflict => (409, "credential_trust_revision_conflict"),
        A::CredentialTrustRevisionExhausted => (409, "credential_trust_revision_exhausted"),
        A::ParticipantCredentialRejected(value) => (
            422,
            match value {
                C::LimitExceeded => "participant_credential_limit_exceeded",
                C::MalformedCertificate => "participant_credential_malformed_certificate",
                C::UnsupportedCertificate => "participant_credential_unsupported_certificate",
                C::UntrustedIssuer => "participant_credential_untrusted_issuer",
                C::NotYetValid => "participant_credential_not_yet_valid",
                C::Expired => "participant_credential_expired",
                C::MalformedCrl => "participant_credential_malformed_crl",
                C::UnsupportedCrl => "participant_credential_unsupported_crl",
                C::UntrustedCrl => "participant_credential_untrusted_crl",
                C::CrlNotYetValid => "participant_credential_crl_not_yet_valid",
                C::CrlExpired => "participant_credential_crl_expired",
                C::Revoked => "participant_credential_revoked",
                C::InvalidSignature => "participant_credential_invalid_signature",
            },
        ),
        A::Domain(value) => (
            422,
            match value {
                D::InvalidTypedParticipantValue(_) => "invalid_typed_participant_value",
                D::InvalidSubjectRevision => "invalid_subject_revision",
                D::SubjectRevisionExhausted => "subject_revision_exhausted",
                D::ParticipantSupportDigestMismatch => "participant_support_digest_mismatch",
                D::ParticipantSubjectKindMismatch => "participant_subject_kind_mismatch",
                _ => return Err(error),
            },
        ),
        _ => return Err(error),
    };
    let message = if matches!(error, A::Domain(D::InvalidTypedParticipantValue(_))) {
        "invalid typed participant value".into()
    } else {
        error.to_string()
    };
    Ok(ApiError {
        status: StatusCode::from_u16(status).expect("fixed HTTP error status"),
        code,
        message,
    })
}
