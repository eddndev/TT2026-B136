use super::State;
use application::{
    document_signing::{ManifestTimestamping, TokenFacts, TsaAuthority},
    ApplicationError,
};
use domain::crypto::Sha256Digest;
use std::sync::{Arc, Mutex};

/// The time-stamping anchor of the synthetic processor: the trust root bytes
/// `root` in a PEM certificate block.
pub const ROOT_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----\ncm9vdA==\n-----END CERTIFICATE-----\n";
/// The synthetic manifest token.
pub const MANIFEST_TOKEN: &[u8] = b"synthetic manifest token";

/// How the synthetic manifest time-stamp fails, when it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFailure {
    Rejected,
    Configuration,
}

pub struct Timestamps(pub Arc<Mutex<State>>);
impl ManifestTimestamping for Timestamps {
    fn facts(&self, token: &[u8], digest: &Sha256Digest) -> Result<TokenFacts, ApplicationError> {
        assert_eq!(token, digest.as_bytes());
        Ok(TokenFacts {
            token: token.to_vec(),
            gen_time: self.0.lock().unwrap().document_gen_time,
            accuracy_seconds: 1,
        })
    }
    fn stamp(
        &self,
        digest: &Sha256Digest,
        anchor_pem: &[u8],
        authority: &TsaAuthority<'_>,
    ) -> Result<TokenFacts, ApplicationError> {
        assert_eq!(anchor_pem, ROOT_PEM);
        assert_eq!(
            (authority.root_der, authority.crl_der),
            (b"root".as_slice(), b"crl".as_slice())
        );
        let mut state = self.0.lock().unwrap();
        state.tsa_calls += 1;
        state.manifest_digest = Some(*digest);
        match state.manifest_failure {
            Some(ManifestFailure::Rejected) => Err(ApplicationError::TimestampEvidenceRejected(
                "synthetic manifest token".into(),
            )),
            Some(ManifestFailure::Configuration) => Err(ApplicationError::InvalidConfiguration(
                "synthetic time-stamping CRL".into(),
            )),
            None => Ok(TokenFacts {
                token: MANIFEST_TOKEN.to_vec(),
                gen_time: state.manifest_gen_time,
                accuracy_seconds: 1,
            }),
        }
    }
}
