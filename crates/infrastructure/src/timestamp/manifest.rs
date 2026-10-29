//! Time-stamping of the individual signing evidence manifest.
//!
//! A token is requested once through the configured authority, verified by
//! the OpenSSL-backed verifier against the configured anchor, and then read
//! with the pure verifier of `crates/evidence-verify` for its `genTime`,
//! accuracy and signer certificate. The certificate and the captured
//! credential trust CRL are checked with the same pure functions the
//! browser validator applies. Rationale: the section "Generation inside the
//! acceptance operation" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

#[cfg(test)]
mod tests;

use application::{
    document_signing::{ManifestTimestamping, TokenFacts, TsaAuthority},
    ApplicationError,
};
use domain::crypto::{Sha256Digest, TimestampService, TimestampVerification, TimestampVerifier};
use evidence_verify::{certificate, crl, policy::DEFAULT_ADMITTED_POLICY, timestamp};

/// [`ManifestTimestamping`] over a timestamp service and its verifier.
pub struct ManifestTimestampAuthority {
    service: Box<dyn TimestampService + Send + Sync>,
    verifier: Box<dyn TimestampVerifier + Send + Sync>,
}

impl ManifestTimestampAuthority {
    pub fn new(
        service: Box<dyn TimestampService + Send + Sync>,
        verifier: Box<dyn TimestampVerifier + Send + Sync>,
    ) -> Self {
        Self { service, verifier }
    }
}

impl ManifestTimestamping for ManifestTimestampAuthority {
    fn facts(&self, token: &[u8], digest: &Sha256Digest) -> Result<TokenFacts, ApplicationError> {
        facts_of(token, digest)
    }

    fn stamp(
        &self,
        digest: &Sha256Digest,
        anchor_pem: &[u8],
        authority: &TsaAuthority<'_>,
    ) -> Result<TokenFacts, ApplicationError> {
        let token = self.service.request(digest)?;
        match self.verifier.verify(&token, digest, anchor_pem)? {
            TimestampVerification::Valid { .. } => {}
            outcome => {
                return Err(ApplicationError::TimestampEvidenceRejected(format!(
                    "{outcome:?}"
                )))
            }
        }
        let facts = verified(&token, digest)?;
        check_authority(
            &facts.signer_certificate_der,
            facts.gen_time,
            facts.accuracy_seconds,
            authority,
        )?;
        Ok(TokenFacts {
            token,
            gen_time: facts.gen_time,
            accuracy_seconds: facts.accuracy_seconds,
        })
    }
}

fn verified(
    token: &[u8],
    digest: &Sha256Digest,
) -> Result<timestamp::TokenFacts, ApplicationError> {
    timestamp::verify(token, digest.as_bytes(), &[DEFAULT_ADMITTED_POLICY]).map_err(|finding| {
        ApplicationError::TimestampEvidenceRejected(format!("{}: {}", finding.code, finding.detail))
    })
}

fn facts_of(token: &[u8], digest: &Sha256Digest) -> Result<TokenFacts, ApplicationError> {
    let facts = verified(token, digest)?;
    Ok(TokenFacts {
        token: token.to_vec(),
        gen_time: facts.gen_time,
        accuracy_seconds: facts.accuracy_seconds,
    })
}

/// Checks that `authority.root_der` issued the time-stamping certificate
/// `signer_der`, that it is valid over `genTime` plus or minus the accuracy,
/// and that `authority.crl_der` is a CRL of at most 1 MiB issued by that
/// root, covering the same interval and not listing the certificate.
fn check_authority(
    signer_der: &[u8],
    gen_time: i64,
    accuracy_seconds: i64,
    authority: &TsaAuthority<'_>,
) -> Result<(), ApplicationError> {
    let root = certificate::parse(authority.root_der).map_err(configuration)?;
    let signer = certificate::parse(signer_der).map_err(configuration)?;
    certificate::check_issued_by(&signer, &root).map_err(configuration)?;
    let from = gen_time.saturating_sub(accuracy_seconds);
    let to = gen_time.saturating_add(accuracy_seconds);
    if !certificate::valid_over(&signer, from, to) {
        return Err(refused(
            "the time-stamping certificate is not valid at genTime",
        ));
    }
    if authority.crl_der.len() > crl::CRL_LIMIT {
        return Err(refused("the credential trust CRL exceeds 1 MiB"));
    }
    let list = crl::parse(authority.crl_der).map_err(configuration)?;
    crl::check_issued_by(&list, &root).map_err(configuration)?;
    if !crl::covers(&list, from, to) {
        return Err(refused("the credential trust CRL does not cover genTime"));
    }
    if crl::revocation_of(&list, &signer.serial).is_some() {
        return Err(refused(
            "the credential trust CRL lists the time-stamping certificate",
        ));
    }
    Ok(())
}

fn configuration(finding: evidence_verify::Finding) -> ApplicationError {
    ApplicationError::InvalidConfiguration(format!(
        "time-stamping authority check failed: {}: {}",
        finding.code, finding.detail
    ))
}

fn refused(reason: &str) -> ApplicationError {
    ApplicationError::InvalidConfiguration(format!(
        "time-stamping authority check failed: {reason}"
    ))
}
