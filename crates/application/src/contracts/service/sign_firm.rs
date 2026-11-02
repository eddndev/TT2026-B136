//! The counter-signature: the Client certificate must not be revoked in the
//! current CRL, then Td over the PDF, the manifest over every covered entry
//! and Tf over the manifest, with `T1 <= Td <= Tf` and Tf before the
//! expiry. The Client binding is not consulted.

use domain::crypto::CredentialFailure;

use super::{
    sign::Signed,
    sign_rules::{token_before, within},
    ContractService, Error,
};
use crate::{
    contracts::{
        model::{Completion, ContractReceipt},
        package::{PackageSource, PartyMaterial},
    },
    document_signing as signing, ApplicationError,
};

impl ContractService {
    pub(super) fn sign_firm(
        &self,
        signed: Signed<'_>,
    ) -> Result<ContractReceipt, ApplicationError> {
        let record = signed.record;
        let fields = *record.request.fields();
        let now = self.now().unix_timestamp();
        if let Err(CredentialFailure::Revoked) = self.ports.verifier.certificate_status(
            &record.client_binding.check.certificate.der,
            &signed.trust,
            now,
        ) {
            return Err(Error::CounterpartyRevoked.into());
        }
        let first = record.signature(1).ok_or(Error::OutOfTurn)?;
        let t1 = first.timestamp.as_ref().ok_or(Error::Inconsistent)?;
        let t1_facts = self
            .ports
            .timestamps
            .facts(&t1.token, &self.ports.hasher.hash_bytes(&first.signature))
            .map_err(|error| match error {
                ApplicationError::InvalidConfiguration(_) => error,
                _ => Error::TimestampUnavailable.into(),
            })?;
        let document = self
            .ports
            .store
            .request_document(signed.principal, record.id())?;
        if document.digest != fields.content_digest
            || document.version != fields.document.version
            || document.id != fields.document.id
        {
            return Err(Error::Inconsistent.into());
        }
        let pdf = self.ports.processor.content_plaintext(&document)?;
        if self.ports.hasher.hash_bytes(&pdf) != fields.content_digest {
            return Err(Error::Inconsistent.into());
        }
        let (td, chain) = self.stamp(&fields.content_digest, &signed.trust)?;
        if td.gen_time < t1_facts.gen_time {
            return Err(Error::TimestampUnavailable.into());
        }
        let source = PackageSource {
            pdf: &pdf,
            request: record,
            root_der: &signed.trust.inspection.root_der,
            t1,
            parties: vec![
                PartyMaterial {
                    statement: first.statement.canonical_bytes().to_vec(),
                    signature: &first.signature,
                    crl_der: &first.crl_der,
                    binding: &record.client_binding,
                    operation: first.statement.operation(),
                },
                PartyMaterial {
                    statement: signed.statement.canonical_bytes().to_vec(),
                    signature: &signed.signature,
                    crl_der: &signed.trust.inspection.crl_der,
                    binding: &record.firm_binding,
                    operation: signed.operation,
                },
            ],
        };
        let entries = source.covered_entries(&td.token, &chain)?;
        let manifest = source.manifest(&entries, record.firm_role)?;
        let (tf, tf_chain) = self.stamp(&self.ports.hasher.hash_bytes(&manifest), &signed.trust)?;
        if tf.gen_time < td.gen_time || tf_chain != chain {
            return Err(Error::TimestampUnavailable.into());
        }
        within(&tf, fields.created_at_seconds, fields.expires_at_seconds)?;
        let accepted = self.now();
        token_before(&tf, accepted.unix_timestamp())?;
        if accepted.unix_timestamp() >= fields.expires_at_seconds {
            return Err(Error::Expired.into());
        }
        signing::check_time(&signed.check, accepted.unix_timestamp())?;
        let completion = Completion {
            document_token: td.token,
            manifest,
            manifest_token: tf.token,
            tsa_chain_pem: chain,
            completed_at: accepted,
        };
        self.commit_signature(signed, accepted, None, Some(completion))
    }
}
