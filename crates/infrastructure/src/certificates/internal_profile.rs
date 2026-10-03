//! Shared mechanics for explicitly selected internal certificate profiles.

mod material;
mod profile;
mod revocation;

use domain::crypto::{
    CredentialCertificate, CredentialCheck, CredentialFailure, CredentialTrustInspection,
    Sha256Digest, Signature,
};

use material::{bounds, signature_matches, CERTIFICATE_LIMIT, CRL_LIMIT};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Profile {
    Root,
    Declaration,
    Partner,
}

pub(super) struct Verifier(Profile);

impl Verifier {
    pub const fn new(profile: Profile) -> Self {
        Self(profile)
    }

    pub fn inspect_certificate(
        &self,
        certificate: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        Ok(profile::certificate(certificate, self.0)?.inspection())
    }

    pub fn inspect_trust(
        &self,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialTrustInspection, CredentialFailure> {
        bounds(root, CERTIFICATE_LIMIT)?;
        bounds(crl, CRL_LIMIT)?;
        let root = profile::certificate(root, Profile::Root)?;
        root.check_time(at)?;
        let crl = revocation::inspect(crl, &root, at)?;
        Ok(crl.trust(&root))
    }

    pub fn verify(
        &self,
        statement_digest: &Sha256Digest,
        certificate: &[u8],
        signature: &Signature,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialCheck, CredentialFailure> {
        bounds(certificate, CERTIFICATE_LIMIT)?;
        bounds(root, CERTIFICATE_LIMIT)?;
        bounds(crl, CRL_LIMIT)?;
        if signature.as_bytes().len() != 384 {
            return Err(CredentialFailure::InvalidSignature);
        }
        let leaf = profile::certificate(certificate, self.0)?;
        let root = profile::certificate(root, Profile::Root)?;
        leaf.check_issuer(&root)?;
        leaf.check_time(at)?;
        root.check_time(at)?;
        let crl = revocation::inspect(crl, &root, at)?;
        if crl.revoked(&leaf.certificate.tbs_certificate.serial_number) {
            return Err(CredentialFailure::Revoked);
        }
        if !signature_matches(&leaf.key, statement_digest, signature.as_bytes()) {
            return Err(CredentialFailure::InvalidSignature);
        }
        let trust = crl.trust(&root);
        Ok(CredentialCheck {
            valid_from: leaf.not_before.max(trust.valid_from),
            valid_until: leaf.not_after.min(trust.valid_until),
            certificate: leaf.inspection(),
            trust,
            statement_digest: *statement_digest,
            signature: signature.clone(),
            checked_at: at,
        })
    }
}
