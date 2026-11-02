//! An in-memory contract store holding one contract with one request.
use super::{client, client_binding, firm, CREATED, DAY};
use crate::document_signing_support::{self as signing, at, Hasher};
use application::{contracts::*, documents::DocumentRecord, identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    document_signing::contract::{ContractRequest, ContractRequestFields},
    identity::{Role, UserId},
};
use std::sync::Mutex;
use uuid::Uuid;

pub const REQUEST: u128 = 100;

#[derive(Default)]
pub struct StoreState {
    pub snapshot: Option<ContractSnapshot>,
    pub receipts: Vec<ContractReceipt>,
    pub commands: Vec<ContractCommand>,
    pub candidates: Option<RequestCandidates>,
    pub exhausted: Vec<ContractSnapshot>,
    pub conflict_on_commit: bool,
}

#[derive(Default)]
pub struct FakeStore(pub Mutex<StoreState>);

pub fn fields() -> ContractRequestFields {
    let trust = signing::trust();
    ContractRequestFields {
        deployment: trust.deployment_id,
        request: Uuid::from_u128(REQUEST),
        case: CaseId::from_uuid(Uuid::from_u128(3)),
        document: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(4)),
            version: DocumentVersion::new(1).unwrap(),
        },
        content_digest: Sha256Digest::from_array([4; 32]),
        client: client().id,
        firm_signer: firm().id,
        requester: firm().id,
        client_leaf: client_binding().statement.material().leaf(),
        firm_leaf: signing::receipt().statement.material().leaf(),
        created_at_seconds: CREATED,
        expires_at_seconds: CREATED + DAY,
    }
}

fn event(revision: u32, kind: EventKind, request: bool) -> ContractEvent {
    ContractEvent {
        revision,
        kind,
        request: request.then(|| Uuid::from_u128(REQUEST)),
        actor: Some(firm().id),
        actor_label: firm().email,
        reason: None,
        at: at(CREATED),
        operation: Uuid::from_u128(u128::from(revision) + 900),
    }
}

/// A contract whose request has the events `kinds` after `created`.
pub fn snapshot(kinds: &[EventKind]) -> ContractSnapshot {
    let mut events = vec![event(1, EventKind::Created, false)];
    for (index, kind) in kinds.iter().enumerate() {
        events.push(event(index as u32 + 2, *kind, true));
    }
    ContractSnapshot {
        root: ContractRoot {
            id: Uuid::from_u128(50),
            case: CaseId::from_uuid(Uuid::from_u128(3)),
            document: DocumentId::from_uuid(Uuid::from_u128(4)),
            title: "Servicios".into(),
            created_by: firm().id,
            created_by_email: firm().email,
            created_at: at(CREATED),
        },
        events,
        requests: vec![RequestRecord {
            request: ContractRequest::new(fields()).unwrap(),
            requester_email: firm().email,
            ceremony_reference: "HF-1".into(),
            subject: None,
            created_at: at(CREATED),
            client_binding: client_binding(),
            firm_binding: signing::receipt(),
            client_role: Role::Client,
            firm_role: Role::Litigator,
            signatures: Vec::new(),
            completion: None,
        }],
        case_scope: true,
        latest_version: 1,
        case_closed: false,
    }
}

impl FakeStore {
    pub fn with(snapshot: ContractSnapshot) -> Self {
        Self(Mutex::new(StoreState {
            snapshot: Some(snapshot),
            ..StoreState::default()
        }))
    }

    pub fn commits(&self) -> usize {
        self.0.lock().unwrap().commands.len()
    }

    pub fn state(&self) -> std::sync::MutexGuard<'_, StoreState> {
        self.0.lock().unwrap()
    }
}

fn receipt(command: &ContractCommand, snapshot: &ContractSnapshot) -> ContractReceipt {
    let (kind, request, signature) = match &command.write {
        ContractWrite::Create { .. } => (EventKind::Created, None, None),
        ContractWrite::Abandon { .. } => (EventKind::Abandoned, None, None),
        ContractWrite::Request { request, .. } => {
            (EventKind::Requested, Some(request.fields().request), None)
        }
        ContractWrite::Close { kind, request, .. } => (*kind, Some(*request), None),
        ContractWrite::Sign {
            request,
            statement,
            signature,
            check,
            timestamp,
            completion,
            ..
        } => (
            if completion.is_some() {
                EventKind::Completed
            } else {
                EventKind::ClientSigned
            },
            Some(*request),
            Some(PartySignature {
                statement: *statement,
                signature: signature.clone(),
                checked_at: check.checked_at,
                valid_from: check.valid_from,
                valid_until: check.valid_until,
                accepted_at: command.at,
                timestamp: timestamp.clone(),
                root_der: b"root".to_vec(),
                crl_der: b"crl".to_vec(),
            }),
        ),
    };
    ContractReceipt {
        contract: snapshot.root.id,
        case: snapshot.root.case,
        event: ContractEvent {
            revision: command.revision,
            kind,
            request,
            actor: command.actor.as_ref().map(|actor| actor.id),
            actor_label: command
                .actor
                .as_ref()
                .map_or(EXPIRY_ACTOR.to_owned(), |actor| actor.email.clone()),
            reason: None,
            at: command.at,
            operation: command.operation,
        },
        request_statement: None,
        signature,
        manifest_sha256: None,
    }
}

impl ContractStore for FakeStore {
    fn case_contracts(
        &self,
        _: &Principal,
        _: CaseId,
    ) -> Result<CaseContractsListing, ApplicationError> {
        unreachable!()
    }
    fn contract(
        &self,
        _: &Principal,
        _: CaseId,
        _: Uuid,
    ) -> Result<Option<ContractSnapshot>, ApplicationError> {
        Ok(self.state().snapshot.clone())
    }
    fn contract_by_request(
        &self,
        _: &Principal,
        _: Uuid,
    ) -> Result<Option<ContractSnapshot>, ApplicationError> {
        Ok(self.state().snapshot.clone())
    }
    fn party_contracts(&self, _: &Principal) -> Result<Vec<ContractSnapshot>, ApplicationError> {
        Ok(self.state().snapshot.clone().into_iter().collect())
    }
    fn receipt(
        &self,
        _: &Principal,
        operation: Uuid,
    ) -> Result<Option<ContractReceipt>, ApplicationError> {
        Ok(self
            .state()
            .receipts
            .iter()
            .find(|r| r.event.operation == operation)
            .cloned())
    }
    fn request_candidates(
        &self,
        _: &Principal,
        _: Uuid,
        _: &str,
        _: UserId,
    ) -> Result<RequestCandidates, ApplicationError> {
        Ok(self.state().candidates.clone().expect("candidates"))
    }
    fn request_document(&self, _: &Principal, _: Uuid) -> Result<DocumentRecord, ApplicationError> {
        unreachable!("these tests read no content")
    }
    fn commit(&self, command: ContractCommand) -> Result<ContractCommit, ApplicationError> {
        let mut state = self.state();
        if state.conflict_on_commit {
            return Err(ContractWorkflowError::RevisionConflict.into());
        }
        let snapshot = state
            .snapshot
            .clone()
            .or_else(|| state.exhausted.first().cloned())
            .expect("a contract");
        let value = receipt(&command, &snapshot);
        state.commands.push(command);
        state.receipts.push(value.clone());
        Ok(ContractCommit::Applied(value))
    }
    fn record_read(
        &self,
        _: &Principal,
        _: Uuid,
        _: ContractRead,
        _: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
    fn exhausted(&self, _: i64, limit: usize) -> Result<Vec<ContractSnapshot>, ApplicationError> {
        assert_eq!(limit, EXPIRY_BATCH);
        Ok(self.state().exhausted.clone())
    }
}

/// The hash the fake hasher gives the request bytes of [`snapshot`].
pub fn request_digest() -> Sha256Digest {
    Hasher.hash_bytes(&ContractRequest::new(fields()).unwrap().canonical_bytes())
}
