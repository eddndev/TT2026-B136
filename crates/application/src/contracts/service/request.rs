//! Creating a signature request. The Client is resolved by the exact
//! e-mail inside this command; there is no account lookup route.

use domain::document_signing::Uuid;
use domain::{
    cases::CaseId,
    crypto::{DocumentVersion, DocumentVersionRef},
    document_signing::contract::{ContractRequest, ContractRequestFields},
    identity::{Role, UserId},
};

use super::{
    bounded_text, expect_revision, identity, lifecycle::require_draft, ContractService, Error,
};
use crate::{
    contracts::{
        access::Relation,
        error::ContractBlockers,
        model::{ContractReceipt, EventKind},
        notices,
        port::{ContractCommand, ContractWrite, RequestCandidates},
    },
    document_signing::{SigningBindingReceipt, SigningWorkflowError},
    documents::{StageDocumentFormat, StageSupportReadLimits},
    ApplicationError,
};

const CEREMONY_LIMIT: usize = 80;
const EMAIL_LIMIT: usize = 320;
const DAY: i64 = 86_400;
const MAX_DAYS: u8 = 30;

/// The fields of a new request as the requester submits them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRequest {
    pub request: Uuid,
    pub operation: Uuid,
    pub expected_revision: u32,
    pub version: u32,
    pub client_email: String,
    pub firm_signer: UserId,
    pub ttl_days: u8,
    pub ceremony_reference: String,
    pub subject: Option<(Uuid, u64)>,
}

impl ContractService {
    pub fn create_request(
        &self,
        token: &str,
        case: CaseId,
        contract: Uuid,
        command: &NewRequest,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        let snapshot = self
            .ports
            .store
            .contract(&principal, case, contract)?
            .ok_or(Error::NotFound)?;
        Relation::of(&principal, &snapshot, None).manage()?;
        identity(command.request)?;
        identity(command.operation)?;
        if let Some(receipt) = self.find_receipt(&principal, command.operation)? {
            if receipt.event.kind != EventKind::Requested
                || receipt.event.request != Some(command.request)
            {
                return Err(SigningWorkflowError::Conflict.into());
            }
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let ceremony = bounded_text(&command.ceremony_reference, CEREMONY_LIMIT)?;
        let email = command.client_email.trim();
        if !(1..=MAX_DAYS).contains(&command.ttl_days)
            || email.is_empty()
            || email.len() > EMAIL_LIMIT
            || command.firm_signer.as_uuid().is_nil()
            || command
                .subject
                .is_some_and(|(id, _): (Uuid, u64)| id.is_nil())
        {
            return Err(Error::Input.into());
        }
        let version = DocumentVersion::new(command.version).map_err(|_| Error::Input)?;
        let at = self.now();
        let mut revision = expect_revision(&snapshot, command.expected_revision)?;
        require_draft(&snapshot, at.unix_timestamp())?;
        let candidates = self.ports.store.request_candidates(
            &principal,
            contract,
            email,
            command.firm_signer,
        )?;
        let (client, firm, document) = self.ready(&candidates, version)?;
        let trust = candidates.trust.as_ref().ok_or(Error::Inconsistent)?;
        let created = at.unix_timestamp();
        let request = ContractRequest::new(ContractRequestFields {
            deployment: trust.deployment_id,
            request: command.request,
            case,
            document: DocumentVersionRef {
                id: snapshot.root.document,
                version,
            },
            content_digest: document,
            client: client.0,
            firm_signer: firm.0,
            requester: principal.id,
            client_leaf: client.1.statement.material().leaf(),
            firm_leaf: firm.1.statement.material().leaf(),
            created_at_seconds: created,
            expires_at_seconds: created + i64::from(command.ttl_days) * DAY,
        })
        .map_err(|_| Error::PartyIneligible(blocked_client()))?;
        let expire_first = self.expiry_step(&snapshot, revision, created);
        if expire_first.is_some() {
            revision += 1;
        }
        let notices = notices::notices(
            &self.ports.notices,
            EventKind::Requested,
            request.fields(),
            &snapshot.root.title,
        );
        let receipt = self
            .ports
            .store
            .commit(ContractCommand {
                actor: Some(principal.clone()),
                operation: command.operation,
                revision,
                at,
                write: ContractWrite::Request {
                    contract,
                    request,
                    client_binding: client.1.statement.material().binding(),
                    firm_binding: firm.1.statement.material().binding(),
                    ceremony_reference: ceremony,
                    subject: command.subject,
                    expire_first,
                },
                notices,
            })?
            .into_receipt();
        if receipt.event.request != Some(command.request) {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    /// Both parties and the document, or every blocker at once.
    #[allow(clippy::type_complexity)]
    fn ready(
        &self,
        candidates: &RequestCandidates,
        version: DocumentVersion,
    ) -> Result<
        (
            (UserId, SigningBindingReceipt),
            (UserId, SigningBindingReceipt),
            domain::crypto::Sha256Digest,
        ),
        ApplicationError,
    > {
        let trust = candidates.trust.as_ref();
        let live = |binding: &SigningBindingReceipt| {
            binding.withdrawn_at.is_none()
                && trust.is_some_and(|trust| {
                    trust.deployment_id == binding.trust.deployment_id
                        && trust.inspection.root_fingerprint == binding.statement.material().root()
                })
        };
        let client = candidates
            .client
            .clone()
            .filter(|(id, binding)| live(binding) && binding.statement.account().id() == *id);
        let firm = candidates
            .firm_signer
            .clone()
            .filter(|(id, role, binding)| {
                matches!(role, Role::Owner | Role::Litigator)
                    && live(binding)
                    && binding.statement.account().id() == *id
                    && client.as_ref().is_none_or(|(client, _)| client != id)
            })
            .map(|(id, _, binding)| (id, binding));
        let document = match &candidates.document {
            Some(record) if record.version == version && !record.is_sealed() => {
                let formats = self.ports.processor.validate_support_batch(
                    std::slice::from_ref(record),
                    &StageSupportReadLimits::standard(),
                    self.ports.formats.as_ref(),
                )?;
                (formats == [StageDocumentFormat::Pdf]).then_some(record.digest)
            }
            _ => None,
        };
        let blockers = ContractBlockers {
            client_not_ready: client.is_none(),
            firm_signer_not_ready: firm.is_none(),
            document_not_ready: document.is_none(),
        };
        match (client, firm, document) {
            (Some(client), Some(firm), Some(document)) => Ok((client, firm, document)),
            _ => Err(Error::PartyIneligible(blockers).into()),
        }
    }
}

fn blocked_client() -> ContractBlockers {
    ContractBlockers {
        client_not_ready: true,
        ..ContractBlockers::default()
    }
}
