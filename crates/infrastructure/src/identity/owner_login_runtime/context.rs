use application::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    identity::{
        certificate_login::CertificateLoginContext, owner_certificates::OwnerBindingAccount,
        Principal,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    clock::OffsetDateTime,
    crypto::{CertificateSummary, CredentialCertificate, CredentialTrustInspection},
    identity::{Role, UserId},
};

use super::{codec, wire};

impl From<&CertificateLoginContext> for wire::Context {
    fn from(value: &CertificateLoginContext) -> Self {
        let account = &value.account;
        let certificate = &value.certificate;
        let summary = &certificate.summary;
        let trust = &value.trust;
        let inspection = &trust.inspection;
        Self {
            account: wire::Account {
                principal: wire::Principal {
                    id: account.principal.id.to_string(),
                    email: account.principal.email.clone(),
                    role: account.principal.role.as_str().into(),
                },
                active: account.active,
                revision: account.revision,
                auth_generation: account.auth_generation,
            },
            binding_id: value.binding_id.to_string(),
            certificate: wire::Certificate {
                der: STANDARD.encode(&certificate.der),
                fingerprint: certificate.fingerprint.to_hex(),
                summary: wire::Summary {
                    subject: summary.subject.clone(),
                    issuer: summary.issuer.clone(),
                    serial_hex: summary.serial_hex.clone(),
                    not_before_unix: summary.not_before_unix,
                    not_after_unix: summary.not_after_unix,
                },
            },
            trust: wire::Trust {
                deployment_id: trust.deployment_id.to_string(),
                revision: trust.revision.get(),
                inspection: wire::Inspection {
                    root_der: STANDARD.encode(&inspection.root_der),
                    crl_der: STANDARD.encode(&inspection.crl_der),
                    root_fingerprint: inspection.root_fingerprint.to_hex(),
                    crl_digest: inspection.crl_digest.to_hex(),
                    crl_number: inspection.crl_number,
                    crl_this_update: inspection.crl_this_update,
                    crl_next_update: inspection.crl_next_update,
                    valid_from: inspection.valid_from,
                    valid_until: inspection.valid_until,
                },
                published_at: wire::Publication {
                    seconds: trust.published_at.unix_timestamp(),
                    nanoseconds: trust.published_at.nanosecond(),
                },
                published_by: trust.published_by.clone(),
            },
        }
    }
}

impl wire::Context {
    pub(super) fn into_context(self) -> Option<CertificateLoginContext> {
        let principal = self.account.principal;
        if principal.role != "owner" {
            return None;
        }
        let summary = self.certificate.summary;
        let trust = self.trust;
        let inspection = trust.inspection;
        Some(CertificateLoginContext {
            account: OwnerBindingAccount {
                principal: Principal {
                    id: UserId::from_uuid(codec::uuid(&principal.id)?),
                    email: principal.email,
                    role: Role::Owner,
                },
                active: self.account.active,
                revision: self.account.revision,
                auth_generation: self.account.auth_generation,
            },
            binding_id: codec::uuid(&self.binding_id)?,
            certificate: CredentialCertificate {
                der: codec::bytes(&self.certificate.der, 16_384)?,
                fingerprint: codec::digest(&self.certificate.fingerprint)?,
                summary: CertificateSummary {
                    subject: summary.subject,
                    issuer: summary.issuer,
                    serial_hex: summary.serial_hex,
                    not_before_unix: summary.not_before_unix,
                    not_after_unix: summary.not_after_unix,
                },
            },
            trust: CredentialTrustSnapshot {
                deployment_id: codec::uuid(&trust.deployment_id)?,
                revision: CredentialTrustRevision::new(trust.revision).ok()?,
                inspection: CredentialTrustInspection {
                    root_der: codec::bytes(&inspection.root_der, 16_384)?,
                    crl_der: codec::bytes(&inspection.crl_der, 1_048_576)?,
                    root_fingerprint: codec::digest(&inspection.root_fingerprint)?,
                    crl_digest: codec::digest(&inspection.crl_digest)?,
                    crl_number: inspection.crl_number,
                    crl_this_update: inspection.crl_this_update,
                    crl_next_update: inspection.crl_next_update,
                    valid_from: inspection.valid_from,
                    valid_until: inspection.valid_until,
                },
                published_at: OffsetDateTime::from_unix_timestamp(trust.published_at.seconds)
                    .ok()?
                    .replace_nanosecond(trust.published_at.nanoseconds)
                    .ok()?,
                published_by: trust.published_by,
            },
        })
    }
}
