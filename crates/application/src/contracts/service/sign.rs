//! Preparing and submitting a party statement. An exact replay of the
//! operation returns its original receipt before any trust or expiry check;
//! different bytes for the same operation conflict. The time-stamping
//! authority is called outside any transaction, once, never retried.

use domain::document_signing::Uuid;
use domain::{
    crypto::{CredentialCheck, Sha256Digest, Signature},
    document_signing::{
        contract::{ContractPartyStatement, ContractPosition},
        SigningBindingMaterial,
    },
};

use super::{
    expect_revision, identity,
    sign_rules::{compare, same_signature, token_before, turn, within},
    ContractService, Error,
};
use crate::{
    contracts::{
        access::Relation,
        model::{ContractReceipt, ContractSnapshot, EventKind, RequestRecord, StatementToken},
        notices,
        port::{ContractCommand, ContractWrite},
    },
    credential_trust::CredentialTrustSnapshot,
    document_signing::{
        self as signing, SigningBindingReceipt, SigningWorkflowError as Signing, TokenFacts,
        TsaAuthority,
    },
    identity::Principal,
    ApplicationError,
};

/// The exact 303 bytes the caller must sign, built without any write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedContractSigning {
    pub statement: ContractPartyStatement,
    pub revision: u32,
}

/// A signed statement as the party submits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedSignature {
    pub operation: Uuid,
    pub expected_revision: u32,
    pub statement: Vec<u8>,
    pub signature: Vec<u8>,
}

/// The current facts a statement is built from and checked against.
pub(super) struct Expected {
    pub(super) statement: ContractPartyStatement,
    pub(super) trust: CredentialTrustSnapshot,
    pub(super) binding: SigningBindingReceipt,
}

impl ContractService {
    pub fn prepare(
        &self,
        token: &str,
        request: Uuid,
        operation: Uuid,
    ) -> Result<PreparedContractSigning, ApplicationError> {
        let principal = self.principal(token)?;
        identity(operation)?;
        let snapshot = self.by_request(&principal, request)?;
        let record = snapshot.request(request).ok_or(Error::Inconsistent)?;
        let relation = Relation::of(&principal, &snapshot, Some(record));
        let position = if relation.client { 1 } else { 2 };
        relation.sign(position)?;
        let now = self.now().unix_timestamp();
        turn(&snapshot, record, position, now)?;
        let expected = self.expected(&principal, record, position, operation)?;
        self.reauthenticate(token, &principal)?;
        Ok(PreparedContractSigning {
            statement: expected.statement,
            revision: snapshot.revision(),
        })
    }

    pub fn sign(
        &self,
        token: &str,
        request: Uuid,
        submitted: &SubmittedSignature,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        identity(submitted.operation)?;
        let snapshot = self.by_request(&principal, request)?;
        let record = snapshot.request(request).ok_or(Error::Inconsistent)?;
        let relation = Relation::of(&principal, &snapshot, Some(record));
        relation.read_request()?;
        let statement = ContractPartyStatement::from_canonical_bytes(&submitted.statement)
            .map_err(|_| Error::Input)?;
        let signature =
            Signature::from_bytes(submitted.signature.clone()).map_err(|_| Error::Input)?;
        let position = statement.position().number();
        relation.sign(position)?;
        if statement.request() != request
            || statement.operation() != submitted.operation
            || statement.account().id() != principal.id
        {
            return Err(Signing::Conflict.into());
        }
        if let Some(receipt) = self.find_receipt(&principal, submitted.operation)? {
            same_signature(&receipt, request, submitted)?;
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let started = self.now();
        let revision = expect_revision(&snapshot, submitted.expected_revision)?;
        turn(&snapshot, record, position, started.unix_timestamp())?;
        let expected = self.expected(&principal, record, position, submitted.operation)?;
        compare(&statement, &expected.statement)?;
        let check = self.ports.verifier.verify_statement(
            &statement,
            &expected.binding.check.certificate.der,
            &signature,
            &expected.trust,
            started.unix_timestamp(),
        )?;
        if check.certificate.fingerprint != statement.material().leaf()
            || check.statement_digest != self.ports.hasher.hash_bytes(&submitted.statement)
        {
            return Err(Error::Inconsistent.into());
        }
        signing::check_time(&check, started.unix_timestamp())?;
        let signed = Signed {
            principal: &principal,
            snapshot: &snapshot,
            record,
            statement,
            signature: submitted.signature.clone(),
            check,
            trust: expected.trust,
            revision,
            operation: submitted.operation,
        };
        let receipt = if position == 1 {
            self.sign_client(signed)?
        } else {
            self.sign_firm(signed)?
        };
        same_signature(&receipt, request, submitted)?;
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    /// Builds the statement the caller would sign now.
    pub(super) fn expected(
        &self,
        principal: &Principal,
        record: &RequestRecord,
        position: u8,
        operation: Uuid,
    ) -> Result<Expected, ApplicationError> {
        let context = self.ports.bindings.load_context(principal.id)?;
        if context.principal != *principal || context.account.id() != principal.id {
            return Err(Signing::AccountChanged.into());
        }
        let binding = if position == 1 {
            record.client_binding.clone()
        } else {
            record.firm_binding.clone()
        };
        if binding.statement.account().id() != principal.id {
            return Err(Error::Inconsistent.into());
        }
        let live = self
            .ports
            .bindings
            .find_current(principal.id)?
            .filter(|current| current.statement == binding.statement);
        if live.is_none_or(|current| current.withdrawn_at.is_some()) {
            return Err(Signing::InactiveBinding.into());
        }
        if context.account.revision() < binding.statement.account().revision()
            || context.account.generation() < binding.statement.account().generation()
        {
            return Err(Signing::AccountChanged.into());
        }
        let trust = context.trust.ok_or(Signing::TrustUnavailable)?;
        signing::check_trust(self.ports.hasher.as_ref(), &trust)?;
        signing::check_successor(&binding, &trust)?;
        let position = match position {
            1 => ContractPosition::Client,
            _ => {
                let first = record.signature(1).ok_or(Error::OutOfTurn)?;
                let token = first.timestamp.as_ref().ok_or(Error::Inconsistent)?;
                ContractPosition::Firm {
                    signature_digest: self.ports.hasher.hash_bytes(&first.signature),
                    token_digest: self.ports.hasher.hash_bytes(&token.token),
                }
            }
        };
        let material = SigningBindingMaterial::new(
            trust.deployment_id,
            binding.statement.material().binding(),
            trust.inspection.root_fingerprint,
            binding.statement.material().leaf(),
            trust.revision.get(),
        )
        .map_err(|_| Error::Inconsistent)?;
        if material.deployment() != record.request.fields().deployment {
            return Err(Signing::TrustChanged.into());
        }
        let statement = ContractPartyStatement::new(
            context.account,
            material,
            operation,
            record.id(),
            self.ports.hasher.hash_bytes(&record.bytes()),
            record.request.fields().content_digest,
            position,
        )
        .map_err(|_| Error::Inconsistent)?;
        Ok(Expected {
            statement,
            trust,
            binding,
        })
    }

    /// Requests one token over `digest`; any authority failure leaves no
    /// state and reads as `contract_timestamp_unavailable`.
    pub(super) fn stamp(
        &self,
        digest: &Sha256Digest,
        trust: &CredentialTrustSnapshot,
    ) -> Result<(TokenFacts, Vec<u8>), ApplicationError> {
        let anchor = self.anchor(trust)?;
        let facts = self
            .ports
            .timestamps
            .stamp(
                digest,
                &anchor,
                &TsaAuthority {
                    root_der: &trust.inspection.root_der,
                    crl_der: &trust.inspection.crl_der,
                },
            )
            .map_err(|error| match error {
                ApplicationError::InvalidConfiguration(_) => error,
                _ => Error::TimestampUnavailable.into(),
            })?;
        Ok((facts, anchor))
    }

    fn anchor(&self, trust: &CredentialTrustSnapshot) -> Result<Vec<u8>, ApplicationError> {
        let material = self.ports.processor.evidence_material();
        let anchor = material
            .tsa_chain_pem
            .clone()
            .unwrap_or_else(|| material.issuer_certificate_pem.clone());
        signing::check_anchor(&anchor, trust)?;
        Ok(anchor)
    }

    fn sign_client(&self, signed: Signed<'_>) -> Result<ContractReceipt, ApplicationError> {
        let fields = *signed.record.request.fields();
        let digest = self.ports.hasher.hash_bytes(&signed.signature);
        let (t1, chain) = self.stamp(&digest, &signed.trust)?;
        within(&t1, fields.created_at_seconds, fields.expires_at_seconds)?;
        let accepted = self.now();
        token_before(&t1, accepted.unix_timestamp())?;
        if accepted.unix_timestamp() >= fields.expires_at_seconds {
            return Err(Error::Expired.into());
        }
        signing::check_time(&signed.check, accepted.unix_timestamp())?;
        let timestamp = StatementToken {
            token: t1.token,
            tsa_chain_pem: chain,
        };
        self.commit_signature(signed, accepted, Some(timestamp), None)
    }

    pub(super) fn commit_signature(
        &self,
        signed: Signed<'_>,
        at: domain::clock::OffsetDateTime,
        timestamp: Option<StatementToken>,
        completion: Option<crate::contracts::model::Completion>,
    ) -> Result<ContractReceipt, ApplicationError> {
        let kind = if completion.is_some() {
            EventKind::Completed
        } else {
            EventKind::ClientSigned
        };
        let notices = notices::notices(
            &self.ports.notices,
            kind,
            signed.record.request.fields(),
            &signed.snapshot.root.title,
        );
        let receipt = self
            .ports
            .store
            .commit(ContractCommand {
                actor: Some(signed.principal.clone()),
                operation: signed.operation,
                revision: signed.revision,
                at,
                write: ContractWrite::Sign {
                    contract: signed.snapshot.root.id,
                    request: signed.record.id(),
                    statement: signed.statement,
                    signature: signed.signature,
                    check: signed.check,
                    timestamp,
                    completion,
                },
                notices,
            })?
            .into_receipt();
        if receipt.event.kind != kind {
            return Err(Error::Inconsistent.into());
        }
        Ok(receipt)
    }
}

/// A verified submission waiting for its tokens and commit.
pub(super) struct Signed<'a> {
    pub(super) principal: &'a Principal,
    pub(super) snapshot: &'a ContractSnapshot,
    pub(super) record: &'a RequestRecord,
    pub(super) statement: ContractPartyStatement,
    pub(super) signature: Vec<u8>,
    pub(super) check: CredentialCheck,
    pub(super) trust: CredentialTrustSnapshot,
    pub(super) revision: u32,
    pub(super) operation: Uuid,
}
