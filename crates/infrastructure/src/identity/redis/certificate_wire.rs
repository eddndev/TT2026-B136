use application::identity::certificate_login::{
    CertificateMfaChallenge, CertificateSessionProvenance,
};
use application::identity::{LoginChallengeIdentity, SessionIdentity};
use domain::{crypto::Sha256Digest, owner_certificate_login::LoginAccount};
use serde::{Deserialize, Serialize};

use super::wire::{self, object, StoredPrincipal};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Authentication {
    kind: String,
    #[serde(deserialize_with = "object")]
    provenance: Provenance,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    #[serde(deserialize_with = "object")]
    principal: StoredPrincipal,
    auth_generation: u64,
    binding_id: uuid::Uuid,
    deployment_id: uuid::Uuid,
    trust_revision: u32,
    root_fingerprint: String,
    leaf_fingerprint: String,
    crl_digest: String,
    valid_from_unix_seconds: i64,
    valid_until_unix_seconds: i64,
}

impl Authentication {
    pub(super) fn from_origin(origin: &CertificateSessionProvenance) -> Self {
        Self {
            kind: "certificate".into(),
            provenance: Provenance {
                principal: origin.principal.clone().into(),
                auth_generation: origin.auth_generation,
                binding_id: origin.binding_id,
                deployment_id: origin.deployment_id,
                trust_revision: origin.trust_revision,
                root_fingerprint: origin.root_fingerprint.to_hex(),
                leaf_fingerprint: origin.leaf_fingerprint.to_hex(),
                crl_digest: origin.crl_digest.to_hex(),
                valid_from_unix_seconds: origin.valid_from_unix_seconds,
                valid_until_unix_seconds: origin.valid_until_unix_seconds,
            },
        }
    }

    pub(super) fn into_origin(self) -> Option<CertificateSessionProvenance> {
        if self.kind != "certificate" {
            return None;
        }
        let value = self.provenance;
        let origin = CertificateSessionProvenance {
            principal: value.principal.into(),
            auth_generation: value.auth_generation,
            binding_id: value.binding_id,
            deployment_id: value.deployment_id,
            trust_revision: value.trust_revision,
            root_fingerprint: digest(&value.root_fingerprint)?,
            leaf_fingerprint: digest(&value.leaf_fingerprint)?,
            crl_digest: digest(&value.crl_digest)?,
            valid_from_unix_seconds: value.valid_from_unix_seconds,
            valid_until_unix_seconds: value.valid_until_unix_seconds,
        };
        bounds(&origin)?;
        Some(origin)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Challenge {
    version: u8,
    #[serde(deserialize_with = "object")]
    identity: LoginChallengeIdentity,
    #[serde(deserialize_with = "object")]
    authentication: Authentication,
    expires_at_unix_seconds: i64,
}

impl Challenge {
    pub(super) fn from_value(value: &CertificateMfaChallenge) -> Self {
        Self {
            version: 2,
            identity: value.identity.clone(),
            authentication: Authentication::from_origin(&value.provenance),
            expires_at_unix_seconds: value.expires_at_unix_seconds,
        }
    }

    pub(super) fn decode(self, now: i64, expiration: i64) -> Option<CertificateMfaChallenge> {
        if self.version != 2 {
            return None;
        }
        let provenance = self.authentication.into_origin()?;
        let value = CertificateMfaChallenge {
            identity: self.identity,
            provenance,
            expires_at_unix_seconds: self.expires_at_unix_seconds,
        };
        let (from, until, deadline) = challenge_bounds(&value)?;
        if now < from
            || now >= until
            || deadline <= now
            || deadline != expiration
            || deadline.checked_sub(now)? > 300_000
        {
            return None;
        }
        Some(value)
    }
}

pub(super) fn bounds(origin: &CertificateSessionProvenance) -> Option<(i64, i64)> {
    LoginAccount::new(
        origin.principal.id,
        origin.principal.role,
        true,
        origin.auth_generation,
    )
    .ok()?;
    if origin.binding_id.is_nil() || origin.deployment_id.is_nil() || origin.trust_revision == 0 {
        return None;
    }
    let from = wire::milliseconds(origin.valid_from_unix_seconds)?;
    let until = wire::milliseconds(origin.valid_until_unix_seconds)?;
    (from < until).then_some((from, until))
}

pub(super) fn matches(identity: &SessionIdentity, origin: &CertificateSessionProvenance) -> bool {
    identity.principal == origin.principal && identity.auth_generation == origin.auth_generation
}

pub(super) fn challenge_bounds(value: &CertificateMfaChallenge) -> Option<(i64, i64, i64)> {
    let (from, until) = bounds(&value.provenance)?;
    let deadline = wire::milliseconds(value.expires_at_unix_seconds)?;
    if value.identity.user_id != value.provenance.principal.id
        || value.identity.auth_generation != value.provenance.auth_generation
        || deadline > until
    {
        return None;
    }
    Some((from, until, deadline))
}

fn digest(value: &str) -> Option<Sha256Digest> {
    let digest = Sha256Digest::from_hex(value).ok()?;
    (digest.to_hex() == value).then_some(digest)
}
