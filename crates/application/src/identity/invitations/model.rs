use crate::identity::{Principal, UserRecord};
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::Role};
use zeroize::Zeroizing;

use super::{InvitationDeliveryState, InvitationId, Uuid};

pub const INVITATION_TTL_SECONDS: i64 = 86_400;
pub const MAX_INVITATION_PAYLOAD_BYTES: usize = 65_536;

/// Commit-time authority includes the full captured account generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationActor {
    pub principal: Principal,
    pub revision: u64,
    pub auth_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationPurpose {
    Delivery,
    PendingMfa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvitationProtectionContext {
    pub purpose: InvitationPurpose,
    pub id: InvitationId,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationStatus {
    Pending,
    PendingMfa,
    Accepted,
    Expired,
}

/// Owner-facing state never contains tokens, hashes or enrollment material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationSummary {
    pub id: InvitationId,
    pub revision: u64,
    pub operation_id: Uuid,
    pub email: String,
    pub role: Role,
    pub status: InvitationStatus,
    pub delivery: InvitationDeliveryState,
    pub issued_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub accepted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationIssueRequest {
    pub operation_id: Uuid,
    pub email: String,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvitationResendRequest {
    pub operation_id: Uuid,
    pub expected_revision: u64,
}

/// Initial ID equals operation_id. The token is encrypted for Delivery at R1.
pub struct IssueInvitation {
    pub request: InvitationIssueRequest,
    pub token_digest: Sha256Digest,
    pub protected_token: Vec<u8>,
    pub at: OffsetDateTime,
}

/// The new token uses Delivery AAD at expected_revision + 1.
pub struct ResendInvitation {
    pub request: InvitationResendRequest,
    pub token_digest: Sha256Digest,
    pub protected_token: Vec<u8>,
    pub at: OffsetDateTime,
}

/// Contains the first preparation's encrypted password/MFA/recovery material.
/// No plaintext secret or usable user exists at this boundary.
#[derive(Clone)]
pub struct PendingInvitation {
    pub operation_id: Uuid,
    pub request_digest: Sha256Digest,
    pub protected_material: Vec<u8>,
}

#[derive(Clone)]
pub struct InvitationCandidate {
    pub id: InvitationId,
    pub revision: u64,
    pub email: String,
    pub role: Role,
    pub expires_at: OffsetDateTime,
    pub pending: Option<PendingInvitation>,
}

pub struct PrepareInvitation {
    pub id: InvitationId,
    pub revision: u64,
    pub token_digest: Sha256Digest,
    pub pending: PendingInvitation,
    pub at: OffsetDateTime,
}

/// Returned only after the live token and chosen password are verified.
/// Retrying restores the same material from the canonical encrypted pending row.
pub struct InvitationEnrollment {
    pub id: InvitationId,
    pub revision: u64,
    pub operation_id: Uuid,
    pub email: String,
    pub role: Role,
    pub expires_at: OffsetDateTime,
    pub totp_secret_base32: Zeroizing<String>,
    pub otpauth_uri: Zeroizing<String>,
    pub recovery_codes: Vec<Zeroizing<String>>,
}

/// Internal prepared account. Storage creates it only with token consumption,
/// audit and receipt, after rechecking the exact pending revision and expiry.
pub struct FinishInvitation {
    pub id: InvitationId,
    pub revision: u64,
    pub pending_operation_id: Uuid,
    pub operation_id: Uuid,
    pub request_digest: Sha256Digest,
    pub token_digest: Sha256Digest,
    pub user: UserRecord,
    pub at: OffsetDateTime,
}

/// Successful completion supplies no session or MFA secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationReceipt {
    pub id: InvitationId,
    pub revision: u64,
    pub operation_id: Uuid,
    pub user_id: domain::identity::UserId,
    pub email: String,
    pub role: Role,
    pub accepted_at: OffsetDateTime,
}

/// Internal receipt lookup checks exact replay before returning public metadata.
#[derive(Clone)]
pub struct InvitationCompletion {
    pub receipt: InvitationReceipt,
    pub request_digest: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvitationQuery {
    pub after: Option<InvitationId>,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationPage {
    pub items: Vec<InvitationSummary>,
    pub next: Option<InvitationId>,
}
