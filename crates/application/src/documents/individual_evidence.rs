//! Historical public proof for an individual signer, independent of current trust.

use std::fmt;

use domain::{
    crypto::{CredentialCheck, DocumentHasher, InternalDeclarationVerifier, Signature},
    document_signing::{DocumentConsent, SigningBindingMaterial, SigningBindingStatement},
};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use time::OffsetDateTime;

use super::{DocumentRecord, SealedEvidence};
use crate::ApplicationError;

const CERTIFICATE_LIMIT: usize = 16 * 1024;
const CRL_LIMIT: usize = 1024 * 1024;

/// Exact captured public statements and proofs; no private key or current authority.
///
/// The existing sealed evidence contains the content signature, certificate and
/// acceptance trust. This extension retains the independent registration trust.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndividualSigningEvidence {
    pub format_version: u8,
    #[serde(deserialize_with = "bounded::<_, 142>")]
    pub binding_statement: Vec<u8>,
    #[serde(deserialize_with = "bounded::<_, 384>")]
    pub binding_signature: Vec<u8>,
    #[serde(deserialize_with = "bounded::<_, 16384>")]
    pub binding_root_der: Vec<u8>,
    #[serde(deserialize_with = "bounded::<_, 1048576>")]
    pub binding_crl_der: Vec<u8>,
    pub binding_registered_at: i64,
    #[serde(deserialize_with = "bounded::<_, 258>")]
    pub consent_statement: Vec<u8>,
    #[serde(deserialize_with = "bounded::<_, 384>")]
    pub consent_signature: Vec<u8>,
    pub accepted_at: i64,
}

impl IndividualSigningEvidence {
    /// Checks historical registration and consent against the exact document record.
    ///
    /// The caller separately verifies plaintext, its detached signature and TSA
    /// evidence, and checks the consent's case against persisted case association.
    /// This method never substitutes current trust or invalidates history on withdrawal.
    pub fn validate(
        &self,
        record: &DocumentRecord,
        evidence: &SealedEvidence,
        hasher: &dyn DocumentHasher,
        verifier: &dyn InternalDeclarationVerifier,
    ) -> Result<(), ApplicationError> {
        self.bounds(evidence)?;
        let binding = SigningBindingStatement::from_canonical_bytes(&self.binding_statement)
            .map_err(|_| invalid())?;
        let consent = DocumentConsent::from_canonical_bytes(&self.consent_statement)
            .map_err(|_| invalid())?;
        if consent.document().id != record.id
            || consent.document().version != record.version
            || consent.content_digest() != record.digest
            || consent.pdf_signature_digest() != hasher.hash_bytes(&evidence.signature)
        {
            return Err(invalid());
        }
        require_identity(&binding, &consent)?;
        let registration = verify(
            &self.binding_statement,
            &self.binding_signature,
            &evidence.signer_certificate_pem,
            &self.binding_root_der,
            &self.binding_crl_der,
            self.binding_registered_at,
            binding.material(),
            hasher,
            verifier,
        )?;
        if registration.trust.root_der != self.binding_root_der
            || registration.trust.crl_der != self.binding_crl_der
        {
            return Err(invalid());
        }
        let acceptance = verify(
            &self.consent_statement,
            &self.consent_signature,
            &evidence.signer_certificate_pem,
            &evidence.issuer_certificate_pem,
            &evidence.crl_pem,
            self.accepted_at,
            consent.material(),
            hasher,
            verifier,
        )?;
        if registration.certificate != acceptance.certificate
            || registration.trust.root_der != acceptance.trust.root_der
        {
            return Err(invalid());
        }
        if binding.material().trust_revision() == consent.material().trust_revision() {
            if registration.trust != acceptance.trust {
                return Err(invalid());
            }
        } else if acceptance.trust.crl_number <= registration.trust.crl_number
            || acceptance.trust.crl_this_update < registration.trust.crl_this_update
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn bounds(&self, evidence: &SealedEvidence) -> Result<(), ApplicationError> {
        if self.format_version != 1
            || self.binding_statement.len() != 142
            || self.consent_statement.len() != 258
            || self.binding_signature.len() != 384
            || self.consent_signature.len() != 384
            || evidence.signature.len() != 384
            || self.accepted_at < self.binding_registered_at
            || OffsetDateTime::from_unix_timestamp(self.binding_registered_at).is_err()
            || OffsetDateTime::from_unix_timestamp(self.accepted_at).is_err()
        {
            return Err(invalid());
        }
        for (bytes, maximum) in [
            (self.binding_root_der.as_slice(), CERTIFICATE_LIMIT),
            (self.binding_crl_der.as_slice(), CRL_LIMIT),
            (
                evidence.signer_certificate_pem.as_slice(),
                CERTIFICATE_LIMIT,
            ),
            (
                evidence.issuer_certificate_pem.as_slice(),
                CERTIFICATE_LIMIT,
            ),
            (evidence.crl_pem.as_slice(), CRL_LIMIT),
        ] {
            if bytes.is_empty() || bytes.len() > maximum {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

fn require_identity(
    binding: &SigningBindingStatement,
    consent: &DocumentConsent,
) -> Result<(), ApplicationError> {
    let before = binding.material();
    let after = consent.material();
    if binding.account().id() != consent.account().id()
        || binding.account().revision() > consent.account().revision()
        || binding.account().generation() > consent.account().generation()
        || before.deployment() != after.deployment()
        || before.binding() != after.binding()
        || before.root() != after.root()
        || before.leaf() != after.leaf()
        || before.trust_revision() > after.trust_revision()
    {
        return Err(invalid());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify(
    statement: &[u8],
    proof: &[u8],
    certificate: &[u8],
    root: &[u8],
    crl: &[u8],
    at: i64,
    material: &SigningBindingMaterial,
    hasher: &dyn DocumentHasher,
    verifier: &dyn InternalDeclarationVerifier,
) -> Result<CredentialCheck, ApplicationError> {
    let signature = Signature::from_bytes(proof.to_vec()).map_err(|_| invalid())?;
    let digest = hasher.hash_bytes(statement);
    let inspected_certificate = verifier
        .inspect_certificate(certificate)
        .map_err(|_| invalid())?;
    let inspected_trust = verifier
        .inspect_trust(root, crl, at)
        .map_err(|_| invalid())?;
    let check = verifier
        .verify(&digest, certificate, &signature, root, crl, at)
        .map_err(|_| invalid())?;
    let cert = &check.certificate;
    let trust = &check.trust;
    if *cert != inspected_certificate
        || *trust != inspected_trust
        || cert.der.is_empty()
        || cert.der.len() > CERTIFICATE_LIMIT
        || trust.root_der.is_empty()
        || trust.root_der.len() > CERTIFICATE_LIMIT
        || trust.crl_der.is_empty()
        || trust.crl_der.len() > CRL_LIMIT
        || cert.fingerprint != hasher.hash_bytes(&cert.der)
        || trust.root_fingerprint != hasher.hash_bytes(&trust.root_der)
        || trust.crl_digest != hasher.hash_bytes(&trust.crl_der)
        || material.leaf() != cert.fingerprint
        || material.root() != trust.root_fingerprint
        || check.statement_digest != digest
        || check.signature != signature
        || check.checked_at != at
        || check.valid_from != cert.summary.not_before_unix.max(trust.valid_from)
        || check.valid_until != cert.summary.not_after_unix.min(trust.valid_until)
        || at < check.valid_from
        || at > check.valid_until
        || at < trust.crl_this_update
        || at > trust.crl_next_update
        || trust.crl_this_update >= trust.crl_next_update
    {
        return Err(invalid());
    }
    Ok(check)
}

fn invalid() -> ApplicationError {
    ApplicationError::StoredDocumentInconsistent("individual signature evidence is invalid".into())
}

fn bounded<'de, D: Deserializer<'de>, const MAXIMUM: usize>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error> {
    struct Bytes<const MAXIMUM: usize>;
    impl<'de, const MAXIMUM: usize> Visitor<'de> for Bytes<MAXIMUM> {
        type Value = Vec<u8>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a public byte array of at most {MAXIMUM} bytes")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<u8>, A::Error> {
            let mut bytes = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAXIMUM));
            while let Some(byte) = sequence.next_element::<u8>()? {
                if bytes.len() == MAXIMUM {
                    return Err(de::Error::custom(
                        "individual public evidence exceeds its limit",
                    ));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    deserializer.deserialize_seq(Bytes::<MAXIMUM>)
}
