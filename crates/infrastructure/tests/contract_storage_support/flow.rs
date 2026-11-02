//! The writes of each contract step, in the order the storage triggers
//! require: audit first, then the request before its event, and each
//! signature after its own event.
use super::{in_transaction, sign_as_client, Harness, DAY};
use application::identity::Principal;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, DocumentVersionRef, Sha256Digest, TimestampService},
    document_signing::{
        contract::{
            ContractPartyStatement, ContractPosition, ContractRequest, ContractRequestFields,
        },
        SignerAccount,
    },
    identity::UserId,
};
use infrastructure::RingSha256Hasher;
use postgres::Transaction;
use uuid::Uuid;

pub struct Event<'a> {
    pub contract: Uuid,
    pub revision: i64,
    pub kind: &'a str,
    pub request: Option<Uuid>,
    pub actor: Option<&'a Principal>,
    pub reason: Option<&'a str>,
    pub at: i64,
    pub operation: Uuid,
    pub sequence: i64,
}

pub const EXPIRY_ACTOR: &str = "system:contract-expiry";

pub fn insert_event(tx: &mut Transaction<'_>, event: &Event<'_>) -> Result<u64, postgres::Error> {
    let actor = event.actor.map(|actor| actor.id.as_uuid());
    let label = event.actor.map_or(EXPIRY_ACTOR, |actor| &actor.email);
    tx.execute(
        "INSERT INTO contract_events(contract_id,revision,kind,request_id,actor_id,actor_label,
        reason,at_seconds,at_nanoseconds,operation_id,audit_sequence)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,0,$9,$10)",
        &[
            &event.contract,
            &event.revision,
            &event.kind,
            &event.request,
            &actor,
            &label,
            &event.reason,
            &event.at,
            &event.operation,
            &event.sequence,
        ],
    )
}

/// One request, as built by the domain codec.
#[derive(Clone, Copy)]
pub struct Request {
    pub id: Uuid,
    pub request: ContractRequest,
}

impl Request {
    pub fn created(&self) -> i64 {
        self.request.fields().created_at_seconds
    }
    pub fn expires(&self) -> i64 {
        self.request.fields().expires_at_seconds
    }
    pub fn digest(&self) -> Sha256Digest {
        RingSha256Hasher.hash_bytes(&self.request.canonical_bytes())
    }
}

pub fn resource(request: Uuid, revision: i64) -> String {
    format!("contract-request:{request}:revision:{revision}")
}

impl Harness {
    pub fn prepare(&self, created: i64, days: i64) -> Request {
        let id = Uuid::new_v4();
        let record = &self.individual.record;
        let request = ContractRequest::new(ContractRequestFields {
            deployment: self.client_binding.statement.material().deployment(),
            request: id,
            case: CaseId::from_uuid(self.individual.db.case.as_uuid()),
            document: DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            content_digest: record.digest,
            client: self.client.id,
            firm_signer: self.firm().id,
            requester: self.firm().id,
            client_leaf: self.client_binding.statement.material().leaf(),
            firm_leaf: self.firm_binding.statement.material().leaf(),
            created_at_seconds: created,
            expires_at_seconds: created + days * DAY,
        })
        .unwrap();
        Request { id, request }
    }

    /// Writes the request row and its `requested` event.
    pub fn insert_request(&mut self, request: &Request) -> Result<(), postgres::Error> {
        let revision = self.next_revision();
        let firm = self.firm();
        let at = request.created();
        let sequence = self.audit(
            &firm.email,
            "contract.requested",
            &resource(request.id, revision),
            at,
        );
        let contract = self.contract;
        in_transaction(self, |tx| {
            self.insert_request_row(tx, request)?;
            insert_event(
                tx,
                &Event {
                    contract,
                    revision,
                    kind: "requested",
                    request: Some(request.id),
                    actor: Some(&firm),
                    reason: None,
                    at,
                    operation: Uuid::new_v4(),
                    sequence,
                },
            )
        })
        .map(|_| ())
    }

    /// Only the request row; its `requested` event must follow in the same
    /// transaction.
    pub fn insert_request_row(
        &self,
        tx: &mut Transaction<'_>,
        request: &Request,
    ) -> Result<u64, postgres::Error> {
        let fields = *request.request.fields();
        let (contract, firm) = (self.contract, self.firm());
        tx.execute(
                "INSERT INTO contract_signature_requests(request_id,contract_id,case_id,document_id,
                document_version,content_digest,client_id,firm_signer_id,requester_id,requester_email,
                client_binding_id,client_leaf,firm_binding_id,firm_leaf,ceremony_reference,deployment_id,
                statement,statement_digest,created_at_seconds,created_at_nanoseconds,expires_at_seconds)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'HF-2026-001',$15,$16,sha256($16),$17,0,$18)",
                &[
                    &request.id,
                    &contract,
                    &fields.case.as_uuid(),
                    &fields.document.id.as_uuid(),
                    &i64::from(fields.document.version.get()),
                    &&fields.content_digest.as_bytes()[..],
                    &fields.client.as_uuid(),
                    &fields.firm_signer.as_uuid(),
                    &fields.requester.as_uuid(),
                    &firm.email,
                    &self.client_binding.statement.material().binding(),
                    &&fields.client_leaf.as_bytes()[..],
                    &self.firm_binding.statement.material().binding(),
                    &&fields.firm_leaf.as_bytes()[..],
                    &fields.deployment,
                    &&request.request.canonical_bytes()[..],
                    &fields.created_at_seconds,
                    &fields.expires_at_seconds,
                ],
        )
    }

    /// A closing event written by `actor`, or by the expiry actor when None.
    pub fn close(
        &mut self,
        request: &Request,
        kind: &str,
        actor: Option<&Principal>,
        reason: Option<&str>,
        at: i64,
    ) -> Result<(), postgres::Error> {
        let revision = self.next_revision();
        let label = actor.map_or(super::flow::EXPIRY_ACTOR.to_owned(), |a| a.email.clone());
        let action = format!("contract.{kind}");
        let sequence = self.audit(&label, &action, &resource(request.id, revision), at);
        let contract = self.contract;
        in_transaction(self, |tx| {
            insert_event(
                tx,
                &Event {
                    contract,
                    revision,
                    kind,
                    request: Some(request.id),
                    actor,
                    reason,
                    at,
                    operation: Uuid::new_v4(),
                    sequence,
                },
            )
        })
        .map(|_| ())
    }

    /// The exact statement and signature of `position` at `at`.
    pub fn statement(
        &mut self,
        request: &Request,
        operation: Uuid,
        position: ContractPosition,
    ) -> (ContractPartyStatement, Vec<u8>) {
        let (principal, receipt) = match position {
            ContractPosition::Client => (self.client.clone(), self.client_binding.clone()),
            ContractPosition::Firm { .. } => (self.firm(), self.firm_binding.clone()),
        };
        let row = self
            .db()
            .admin
            .query_one(
                "SELECT revision,auth_generation FROM users WHERE id=$1",
                &[&principal.id.as_uuid()],
            )
            .unwrap();
        let account = SignerAccount::from_signed_counters(
            UserId::from_uuid(principal.id.as_uuid()),
            u64::try_from(row.get::<_, i64>(0)).unwrap(),
            u64::try_from(row.get::<_, i64>(1)).unwrap(),
        )
        .unwrap();
        let statement = ContractPartyStatement::new(
            account,
            *receipt.statement.material(),
            operation,
            request.id,
            request.digest(),
            request.request.fields().content_digest,
            position,
        )
        .unwrap();
        let bytes = statement.canonical_bytes();
        let signature = match position {
            ContractPosition::Client => sign_as_client(&bytes),
            ContractPosition::Firm { .. } => crate::individual_signing_fixture::sign(&bytes),
        };
        (statement, signature.as_bytes().to_vec())
    }

    /// A real time-stamp response from the fixture authority over `digest`.
    pub fn token(digest: &Sha256Digest) -> Vec<u8> {
        crate::individual_document_backend_support::processor::authority()
            .request(digest)
            .unwrap()
    }
}
