//! A case with one pending PDF version, a Litigator firm signer and an
//! invited Client, each holding a live document-signing binding on a
//! different certificate, for contract storage tests:
//! docs/adr/0086-bilateral-contract-signing.md.
pub mod flow;
pub mod signing;

use crate::individual_document_backend_support::Harness as Individual;
use crate::signing_binding_support::{self as binding, Fixture, TOKEN};
use application::{
    document_signing::{SigningBindingReceipt, SigningBindingStore as _},
    identity::Principal,
};
use domain::{audit::AuditLog, clock::OffsetDateTime, crypto::Signature, identity::UserId};
use infrastructure::PostgresAuditLog;
use postgres::{Client, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub const DAY: i64 = 86_400;

pub struct Harness {
    pub individual: Individual,
    pub client: Principal,
    pub client_binding: SigningBindingReceipt,
    pub firm_binding: SigningBindingReceipt,
    pub contract: Uuid,
    audit: PostgresAuditLog,
}

impl Harness {
    /// Returns None without `CASE_TEST_DATABASE_URL`, like every backend fixture.
    pub fn new() -> Option<Self> {
        let mut harness = Self::without_contract()?;
        harness.create_contract();
        Some(harness)
    }

    /// The parties and the document, before any contract row or audit exists.
    pub fn without_contract() -> Option<Self> {
        Self::without_contract_at(crate::declaration_fixture::fixture().at)
    }

    /// The same parties and document, registered at `at`, Unix seconds.
    pub fn without_contract_at(at: i64) -> Option<Self> {
        let mut individual = Individual::new_at(at)?;
        let firm_binding = binding::store(&individual.db, &individual.clock)
            .find(individual.actor.id, individual.binding_id)
            .unwrap()
            .unwrap();
        let id = individual.db.user("client", false);
        let client = binding::principal(&mut individual.db, id);
        let audit = PostgresAuditLog::open(&individual.db.runtime_url).unwrap();
        let mut harness = Self {
            individual,
            client: client.clone(),
            client_binding: firm_binding.clone(),
            firm_binding,
            contract: Uuid::new_v4(),
            audit,
        };
        harness.invite(id);
        harness.client_binding = harness.bind_client();
        Some(harness)
    }

    pub fn db(&mut self) -> &mut Fixture {
        &mut self.individual.db
    }

    pub fn firm(&self) -> Principal {
        self.individual.actor.clone()
    }

    /// The fixture clock in whole seconds; contract instants use no nanoseconds.
    pub fn now(&self) -> i64 {
        use domain::clock::Clock as _;
        self.individual.clock.now().unix_timestamp()
    }

    /// Commits one audit row ahead of the write that names it. A rejected
    /// write therefore leaves an audit row without its event, which the
    /// startup inventory reports, so tests check startup before rejections.
    pub fn audit(&mut self, actor: &str, action: &str, resource: &str, at: i64) -> i64 {
        let at = OffsetDateTime::from_unix_timestamp(at).unwrap();
        let sequence = self
            .audit
            .append(actor, action, resource, at)
            .unwrap()
            .event
            .sequence;
        i64::try_from(sequence).unwrap()
    }

    pub fn runtime(&self) -> Client {
        self.individual.db.runtime()
    }

    /// An accepted Client invitation, the only way a Client becomes eligible.
    fn invite(&mut self, user: UserId) {
        let at = self.now() - DAY;
        let issued = self.audit("owner@example.test", "member.invited", "invitation", at);
        let accepted = self.audit(
            &self.client.email.clone(),
            "member.joined",
            "invitation",
            at,
        );
        let invitation = Uuid::new_v4();
        let token = vec![9_u8; 32];
        self.db()
            .admin
            .execute(
                "INSERT INTO member_invitations(id,email,role,revision,operation_id,issued_at,
                expires_at,accepted_at,token_digest,delivery_state,delivery_generation,audit_sequence)
                SELECT $1,email,'client',1,$1,$2::bigint,$2::bigint+86400,$2::bigint+60,$3,'sent',0,$4 FROM users WHERE id=$5",
                &[&invitation, &at, &token, &issued, &user.as_uuid()],
            )
            .unwrap();
        self.db()
            .admin
            .execute(
                "INSERT INTO member_invitation_completions(invitation_id,operation_id,token_digest,
                request_digest,user_id,receipt,audit_sequence) VALUES($1,$2,$3,$3,$4,'{}'::jsonb,$5)",
                &[&invitation, &Uuid::new_v4(), &token, &user.as_uuid(), &accepted],
            )
            .unwrap();
    }

    fn bind_client(&mut self) -> SigningBindingReceipt {
        let store = binding::store(&self.individual.db, &self.individual.clock);
        let service = binding::service(store, &self.individual.clock, self.client.clone());
        let prepared = service
            .prepare(
                TOKEN,
                Uuid::new_v4(),
                &crate::declaration_fixture::fixture().other_leaf,
            )
            .unwrap();
        let signature = sign_as_client(&prepared.statement().canonical_bytes());
        service
            .register(
                TOKEN,
                prepared.statement(),
                &prepared.certificate().der,
                &signature,
            )
            .unwrap()
    }

    /// The contract root and its `created` event, committed together.
    pub fn create_contract(&mut self) {
        let at = self.now();
        let firm = self.firm();
        let resource = format!(
            "contract:{}:document:{}",
            self.contract,
            self.individual.record.id.as_uuid()
        );
        let sequence = self.audit(&firm.email, "contract.created", &resource, at);
        let mut client = self.runtime();
        let mut tx = client.transaction().unwrap();
        tx.execute(
            "INSERT INTO contracts(id,case_id,document_id,title,created_by,created_by_email,
            created_at_seconds,created_at_nanoseconds) VALUES($1,$2,$3,'Contrato de servicios',$4,$5,$6,0)",
            &[
                &self.contract,
                &self.individual.db.case.as_uuid(),
                &self.individual.record.id.as_uuid(),
                &firm.id.as_uuid(),
                &firm.email,
                &at,
            ],
        )
        .unwrap();
        flow::insert_event(
            &mut tx,
            &flow::Event {
                contract: self.contract,
                revision: 1,
                kind: "created",
                request: None,
                actor: Some(&firm),
                reason: None,
                at,
                operation: Uuid::new_v4(),
                sequence,
            },
        )
        .unwrap();
        tx.commit().unwrap();
    }

    pub fn next_revision(&mut self) -> i64 {
        let contract = self.contract;
        self.db()
            .admin
            .query_one(
                "SELECT coalesce(max(revision),0)+1 FROM contract_events WHERE contract_id=$1",
                &[&contract],
            )
            .unwrap()
            .get(0)
    }

    /// Another pending PDF version in the same case.
    pub fn upload(&self, name: &str) -> application::documents::DocumentRecord {
        use application::documents::CaseDocumentStore as _;
        use domain::clock::Clock as _;
        let record = crate::individual_document_backend_support::processor::processor()
            .prepare(name, crate::individual_signing_fixture::PDF)
            .unwrap();
        infrastructure::PostgresCaseDocumentStore::open(&self.individual.db.runtime_url)
            .unwrap()
            .insert(
                self.individual.actor.id,
                self.individual.db.case,
                record.clone(),
                self.individual.clock.now(),
            )
            .unwrap();
        record
    }

    pub fn arc_clock(&self) -> Arc<binding::Clock> {
        self.individual.clock.clone()
    }
}

/// Signs with the key of the second fixture certificate, the Client's.
pub fn sign_as_client(bytes: &[u8]) -> Signature {
    use std::io::Write;
    let mut input = tempfile::NamedTempFile::new().unwrap();
    input.write_all(bytes).unwrap();
    let key = crate::declaration_fixture::fixture()
        .ca
        .join("private/second-declarant.key.pem");
    Signature::from_bytes(crate::declaration_fixture::openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        key.to_str().unwrap(),
        input.path().to_str().unwrap(),
    ]))
    .unwrap()
}

/// The SQLSTATE and message of a rejected statement.
pub fn rejection<T>(result: Result<T, postgres::Error>) -> (String, String) {
    let error = result.err().expect("the statement must be rejected");
    let error = error.as_db_error().expect("a database rejection");
    (error.code().code().to_owned(), error.message().to_owned())
}

/// True when a runtime connection passes every startup check.
pub fn opens(harness: &Harness) -> bool {
    PostgresAuditLog::open(&harness.individual.db.runtime_url).is_ok()
}

/// Changes stored rows past their triggers, as only a superuser could.
pub fn tamper(harness: &mut Harness, sql: &str) {
    harness
        .db()
        .admin
        .batch_execute(&format!(
            "BEGIN; SET LOCAL session_replication_role=replica; {sql}; COMMIT;"
        ))
        .unwrap();
}

/// Runs `body` in one READ COMMITTED transaction on a fresh runtime connection.
pub fn in_transaction<T>(
    harness: &Harness,
    body: impl FnOnce(&mut Transaction<'_>) -> Result<T, postgres::Error>,
) -> Result<T, postgres::Error> {
    let mut client = harness.runtime();
    let mut tx = client.transaction()?;
    let value = body(&mut tx)?;
    tx.commit()?;
    Ok(value)
}
