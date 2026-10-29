use domain::{
    cases::CaseId,
    crypto::{DocumentVersionRef, Signature},
    document_signing::{DocumentConsent, SigningBindingMaterial, Uuid},
    identity::Role,
};

use super::{
    document_manifest, document_validation as validation, evidence, validate_binding_receipt,
    DocumentSigningPorts, PreparedDocumentSigning, SigningCommit, SigningReceipt,
    SigningWorkflowError as Error, VerifiedDocumentSigning,
};
use crate::{
    documents::{
        reserve_document_name, DocumentAction, ExternalSignerMaterial, IndividualSigningEvidence,
        StageDocumentFormat, StageSupportReadLimits, VersionSelection,
    },
    identity::Principal,
    ApplicationError,
};

pub struct DocumentSigningService {
    pub(super) ports: DocumentSigningPorts,
}

impl DocumentSigningService {
    pub fn new(ports: DocumentSigningPorts) -> Self {
        Self { ports }
    }

    fn principal(&self, token: &str) -> Result<Principal, ApplicationError> {
        let principal = self.ports.identity.authenticate(token)?;
        if principal.role != Role::Litigator {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal)
    }

    fn reauthenticate(&self, token: &str, expected: &Principal) -> Result<(), ApplicationError> {
        if self.principal(token)? != *expected {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    pub fn prepare(
        &self,
        token: &str,
        operation: Uuid,
        case: CaseId,
        document: DocumentVersionRef,
        pdf_signature: &Signature,
    ) -> Result<PreparedDocumentSigning, ApplicationError> {
        let principal = self.principal(token)?;
        let prepared = self.prepare_for(&principal, operation, case, document, pdf_signature)?;
        self.reauthenticate(token, &principal)?;
        Ok(prepared)
    }

    fn prepare_for(
        &self,
        principal: &Principal,
        operation: Uuid,
        case: CaseId,
        document: DocumentVersionRef,
        pdf_signature: &Signature,
    ) -> Result<PreparedDocumentSigning, ApplicationError> {
        validation::references(case, document, operation)?;
        validation::signature(pdf_signature)?;
        let record = self.ports.documents.load(
            principal.id,
            case,
            document.id,
            VersionSelection::Exact(document.version),
            DocumentAction::Seal,
        )?;
        if record.id != document.id || record.version != document.version {
            return Err(Error::Inconsistent.into());
        }
        reserve_document_name(&record.name)?;
        if record.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(
                record.id.to_string(),
            ));
        }
        let formats = self.ports.processor.validate_support_batch(
            std::slice::from_ref(&record),
            &StageSupportReadLimits::standard(),
            self.ports.formats.as_ref(),
        )?;
        if formats != [StageDocumentFormat::Pdf] {
            return Err(Error::Input.into());
        }
        let context = self.ports.bindings.load_context(principal.id)?;
        if context.principal != *principal || context.account.id() != principal.id {
            return Err(Error::AccountChanged.into());
        }
        let binding = self
            .ports
            .bindings
            .find_current(principal.id)?
            .ok_or(Error::InactiveBinding)?;
        validate_binding_receipt(
            self.ports.hasher.as_ref(),
            &binding,
            principal.id,
            binding.statement.material().binding(),
        )?;
        if binding.withdrawn_at.is_some() {
            return Err(Error::InactiveBinding.into());
        }
        if context.account.revision() < binding.statement.account().revision()
            || context.account.generation() < binding.statement.account().generation()
        {
            return Err(Error::AccountChanged.into());
        }
        let trust = context.trust.ok_or(Error::TrustUnavailable)?;
        evidence::trust(self.ports.hasher.as_ref(), &trust)?;
        validation::successor(&binding, &trust)?;
        let material = SigningBindingMaterial::new(
            trust.deployment_id,
            binding.statement.material().binding(),
            trust.inspection.root_fingerprint,
            binding.check.certificate.fingerprint,
            trust.revision.get(),
        )
        .map_err(|_| Error::Inconsistent)?;
        let consent = DocumentConsent::new(
            context.account,
            material,
            operation,
            case,
            document,
            record.digest,
            self.ports.hasher.hash_bytes(pdf_signature.as_bytes()),
        )
        .map_err(|_| Error::Inconsistent)?;
        Ok(PreparedDocumentSigning {
            principal: principal.clone(),
            account: context.account,
            trust,
            binding,
            record,
            consent,
        })
    }

    pub fn submit(
        &self,
        token: &str,
        consent: &DocumentConsent,
        pdf_signature: &Signature,
        consent_signature: &Signature,
    ) -> Result<SigningReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        validation::references(consent.case(), consent.document(), consent.operation())?;
        validation::signature(pdf_signature)?;
        validation::signature(consent_signature)?;
        if consent.account().id() != principal.id {
            return Err(Error::Conflict.into());
        }
        if let Some(receipt) = self.ports.store.find(
            &principal,
            consent.case(),
            consent.document(),
            consent.operation(),
        )? {
            self.receipt(&receipt, &principal, consent)?;
            validation::exact_intent(&receipt, consent, pdf_signature, consent_signature)?;
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let prepared = self.prepare_for(
            &principal,
            consent.operation(),
            consent.case(),
            consent.document(),
            pdf_signature,
        )?;
        if consent.account() != prepared.consent.account() {
            return Err(Error::AccountChanged.into());
        }
        if consent.material().deployment() != prepared.trust.deployment_id
            || consent.material().root() != prepared.trust.inspection.root_fingerprint
            || consent.material().trust_revision() != prepared.trust.revision.get()
        {
            return Err(Error::TrustChanged.into());
        }
        if consent != &prepared.consent {
            return Err(Error::Conflict.into());
        }
        let started = self.ports.clock.now();
        if started < prepared.trust.published_at || started < prepared.binding.registered_at {
            return Err(Error::Inconsistent.into());
        }
        let check = self.ports.verifier.verify_consent(
            consent,
            &prepared.binding.check.certificate.der,
            pdf_signature,
            consent_signature,
            &prepared.trust,
            started.unix_timestamp(),
        )?;
        validation::verified(
            self.ports.hasher.as_ref(),
            &prepared,
            &check,
            consent_signature,
            started.unix_timestamp(),
        )?;
        let before_timestamp = self.ports.clock.now();
        if before_timestamp < started {
            return Err(Error::Inconsistent.into());
        }
        evidence::check_time(&check, before_timestamp.unix_timestamp())?;
        self.reauthenticate(token, &principal)?;
        let material = self.ports.processor.evidence_material();
        let anchor = material
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&material.issuer_certificate_pem);
        document_manifest::check_anchor(anchor, &prepared.trust)?;
        let public = ExternalSignerMaterial {
            certificate_pem: validation::pem("CERTIFICATE", &check.certificate.der),
            issuer_pem: validation::pem("CERTIFICATE", &check.trust.root_der),
            crl_pem: validation::pem("X509 CRL", &check.trust.crl_der),
        };
        let sealed =
            self.ports
                .processor
                .seal_external(&prepared.record, pdf_signature, &public)?;
        let mut record = sealed.record;
        let mut individual = IndividualSigningEvidence {
            format_version: 1,
            binding_statement: prepared.binding.statement.canonical_bytes().to_vec(),
            binding_signature: prepared.binding.check.signature.as_bytes().to_vec(),
            binding_root_der: prepared.binding.trust.inspection.root_der.clone(),
            binding_crl_der: prepared.binding.trust.inspection.crl_der.clone(),
            binding_registered_at: prepared.binding.registered_at.unix_timestamp(),
            consent_statement: consent.canonical_bytes().to_vec(),
            consent_signature: consent_signature.as_bytes().to_vec(),
            accepted_at: 0,
        };
        let generated = document_manifest::generate(
            &self.ports,
            &prepared.trust,
            &record,
            sealed.size,
            &individual,
            consent,
        )?;
        let not_before = self.ports.clock.now();
        if not_before < before_timestamp {
            return Err(Error::Inconsistent.into());
        }
        evidence::check_time(&check, not_before.unix_timestamp())?;
        document_manifest::check_before(&generated.facts, not_before.unix_timestamp())?;
        self.reauthenticate(token, &principal)?;
        individual.accepted_at = not_before.unix_timestamp();
        let evidence = record.evidence.as_mut().ok_or(Error::Inconsistent)?;
        evidence.individual = Some(individual);
        let expected_evidence = evidence.clone();
        let valid_until = check.valid_until;
        let manifest = generated.manifest;
        let committed = self.ports.store.commit(VerifiedDocumentSigning {
            prepared,
            check,
            record,
            not_before,
            manifest: manifest.clone(),
        })?;
        let (receipt, applied) = match committed {
            SigningCommit::Applied(receipt) => (receipt, true),
            SigningCommit::Existing(receipt) => (receipt, false),
        };
        self.receipt(&receipt, &principal, consent)?;
        validation::exact_intent(&receipt, consent, pdf_signature, consent_signature)?;
        let mut returned_evidence = receipt.evidence.clone();
        returned_evidence
            .individual
            .as_mut()
            .ok_or(Error::Inconsistent)?
            .accepted_at = not_before.unix_timestamp();
        if applied
            && (receipt.accepted_at < not_before
                || receipt.accepted_at.unix_timestamp() > valid_until
                || returned_evidence != expected_evidence
                || receipt.manifest.as_ref() != Some(&manifest))
        {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    fn receipt(
        &self,
        receipt: &SigningReceipt,
        principal: &Principal,
        expected: &DocumentConsent,
    ) -> Result<(), ApplicationError> {
        validation::validate_signing_receipt(
            self.ports.hasher.as_ref(),
            receipt,
            principal,
            expected.case(),
            expected.document(),
            expected.operation(),
        )
    }

    pub fn get(
        &self,
        token: &str,
        case: CaseId,
        document: DocumentVersionRef,
        operation: Uuid,
    ) -> Result<SigningReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        validation::references(case, document, operation)?;
        let receipt = self
            .ports
            .store
            .find(&principal, case, document, operation)?
            .ok_or(Error::NotFound)?;
        validation::validate_signing_receipt(
            self.ports.hasher.as_ref(),
            &receipt,
            &principal,
            case,
            document,
            operation,
        )?;
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}
