//! Both party signatures and the completion, each after its own event.
use super::flow::{insert_event, resource, Event, Request};
use super::{in_transaction, Harness};
use application::{document_signing::SigningBindingReceipt, identity::Principal};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    document_signing::contract::{ContractPartyStatement, ContractPosition},
};
use evidence_verify::manifest::contract::{
    encode, AccountRole, ContractManifest, ContractRole, ContractSigner,
};
use infrastructure::RingSha256Hasher;
use postgres::Transaction;
use uuid::Uuid;

fn sha256(bytes: &[u8]) -> Sha256Digest {
    RingSha256Hasher.hash_bytes(bytes)
}

pub struct Signed<'a> {
    pub request: Uuid,
    pub position: i16,
    pub operation: Uuid,
    pub signer: &'a Principal,
    pub binding: &'a SigningBindingReceipt,
    pub statement: &'a ContractPartyStatement,
    pub signature: &'a [u8],
    pub at: i64,
    pub token: Option<&'a [u8]>,
}

pub fn insert_signature(tx: &mut Transaction<'_>, s: &Signed<'_>) -> Result<u64, postgres::Error> {
    let material = s.binding.statement.material();
    let account = s.statement.account();
    let chain = s
        .token
        .map(|_| crate::declaration_fixture::fixture().root.clone());
    tx.execute(
        "INSERT INTO contract_party_signatures(request_id,position,operation_id,signer_id,binding_id,
        leaf_fingerprint,deployment_id,trust_revision,root_fingerprint,account_revision,auth_generation,
        actor_email,statement,statement_digest,signature,checked_at,valid_from,valid_until,
        accepted_at_seconds,accepted_at_nanoseconds,timestamp_token,tsa_chain_pem)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,sha256($13),$14,$15,$16,$17,$15,0,$18,$19)",
        &[
            &s.request,
            &s.position,
            &s.operation,
            &s.signer.id.as_uuid(),
            &material.binding(),
            &&material.leaf().as_bytes()[..],
            &material.deployment(),
            &i64::from(material.trust_revision()),
            &&material.root().as_bytes()[..],
            &i64::try_from(account.revision()).unwrap(),
            &i64::try_from(account.generation()).unwrap(),
            &s.signer.email,
            &&s.statement.canonical_bytes()[..],
            &s.signature,
            &s.at,
            &s.binding.check.valid_from,
            &s.binding.check.valid_until,
            &s.token,
            &chain,
        ],
    )
}

impl Harness {
    /// The Client signature with its event and a real T1 token.
    pub fn sign_client(&mut self, request: &Request, at: i64) -> Result<(), postgres::Error> {
        let operation = Uuid::new_v4();
        let (statement, signature) = self.statement(request, operation, ContractPosition::Client);
        let token = Harness::token(&sha256(&signature));
        let revision = self.next_revision();
        let client = self.client.clone();
        let sequence = self.audit(
            &client.email,
            "contract.client_signed",
            &resource(request.id, revision),
            at,
        );
        let (contract, binding) = (self.contract, self.client_binding.clone());
        in_transaction(self, |tx| {
            insert_event(
                tx,
                &Event {
                    contract,
                    revision,
                    kind: "client_signed",
                    request: Some(request.id),
                    actor: Some(&client),
                    reason: None,
                    at,
                    operation,
                    sequence,
                },
            )?;
            insert_signature(
                tx,
                &Signed {
                    request: request.id,
                    position: 1,
                    operation,
                    signer: &client,
                    binding: &binding,
                    statement: &statement,
                    signature: &signature,
                    at,
                    token: Some(&token),
                },
            )
        })
        .map(|_| ())
    }

    /// The firm signature, its `completed` event and the completion row.
    pub fn sign_firm(&mut self, request: &Request, at: i64) -> Result<(), postgres::Error> {
        let row = self.db().admin.query_one(
            "SELECT signature,timestamp_token,statement,tsa_chain_pem,operation_id
            FROM contract_party_signatures WHERE request_id=$1 AND position=1",
            &[&request.id],
        )?;
        let first: Vec<Vec<u8>> = (0..4).map(|n| row.get(n)).collect();
        let first_operation: Uuid = row.get(4);
        let operation = Uuid::new_v4();
        let position = ContractPosition::Firm {
            signature_digest: sha256(&first[0]),
            token_digest: sha256(&first[1]),
        };
        let (statement, signature) = self.statement(request, operation, position);
        let document_token = Harness::token(&request.request.fields().content_digest);
        let chain = crate::declaration_fixture::fixture().root.clone();
        let shared = [document_token.clone(), chain.clone()];
        let manifest = self.manifest(
            request,
            &first,
            &statement,
            &signature,
            first_operation,
            &shared,
        );
        let manifest_token = Harness::token(&sha256(&manifest));
        let revision = self.next_revision();
        let firm = self.firm();
        let sequence = self.audit(
            &firm.email,
            "contract.completed",
            &resource(request.id, revision),
            at,
        );
        let (contract, binding) = (self.contract, self.firm_binding.clone());
        in_transaction(self, |tx| {
            insert_event(
                tx,
                &Event {
                    contract,
                    revision,
                    kind: "completed",
                    request: Some(request.id),
                    actor: Some(&firm),
                    reason: None,
                    at,
                    operation,
                    sequence,
                },
            )?;
            insert_signature(
                tx,
                &Signed {
                    request: request.id,
                    position: 2,
                    operation,
                    signer: &firm,
                    binding: &binding,
                    statement: &statement,
                    signature: &signature,
                    at,
                    token: None,
                },
            )?;
            tx.execute(
                "INSERT INTO contract_completions(request_id,document_token,manifest,manifest_sha256,
                manifest_token,tsa_chain_pem,completed_at_seconds,completed_at_nanoseconds)
                VALUES($1,$2,$3,sha256($3),$4,$5,$6,0)",
                &[
                    &request.id,
                    &document_token,
                    &manifest,
                    &manifest_token,
                    &chain,
                    &at,
                ],
            )
        })
        .map(|_| ())
    }

    /// A manifest whose stored-byte artifacts are exact; the encoded
    /// certificates and CRLs, chosen by the exporter, are placeholders.
    fn manifest(
        &self,
        request: &Request,
        first: &[Vec<u8>],
        statement: &ContractPartyStatement,
        signature: &[u8],
        first_operation: Uuid,
        shared: &[Vec<u8>; 2],
    ) -> Vec<u8> {
        let fields = request.request.fields();
        let placeholder = b"exporter encoding".to_vec();
        let party = |receipt: &SigningBindingReceipt| {
            [
                (ContractRole::SignerCertificate, placeholder.clone()),
                (ContractRole::IssuerCrl, placeholder.clone()),
                (
                    ContractRole::BindingStatement,
                    receipt.statement.canonical_bytes().to_vec(),
                ),
                (
                    ContractRole::BindingSignature,
                    receipt.check.signature.as_bytes().to_vec(),
                ),
                (
                    ContractRole::BindingRoot,
                    receipt.trust.inspection.root_der.clone(),
                ),
                (
                    ContractRole::BindingCrl,
                    receipt.trust.inspection.crl_der.clone(),
                ),
            ]
        };
        let mut artifacts = vec![
            ContractManifest::artifact(
                0,
                ContractRole::Document,
                crate::individual_signing_fixture::PDF,
            ),
            ContractManifest::artifact(0, ContractRole::DocumentTimestamp, &shared[0]),
            ContractManifest::artifact(
                0,
                ContractRole::Request,
                &request.request.canonical_bytes(),
            ),
            ContractManifest::artifact(0, ContractRole::IssuerCertificate, &placeholder),
            ContractManifest::artifact(0, ContractRole::TsaChain, &shared[1]),
            ContractManifest::artifact(1, ContractRole::Statement, &first[2]),
            ContractManifest::artifact(1, ContractRole::StatementSignature, &first[0]),
            ContractManifest::artifact(1, ContractRole::StatementTimestamp, &first[1]),
            ContractManifest::artifact(1, ContractRole::StatementTsaChain, &first[3]),
            ContractManifest::artifact(2, ContractRole::Statement, &statement.canonical_bytes()),
            ContractManifest::artifact(2, ContractRole::StatementSignature, signature),
        ];
        for (index, receipt) in [&self.client_binding, &self.firm_binding]
            .into_iter()
            .enumerate()
        {
            for (role, bytes) in party(receipt) {
                artifacts.push(ContractManifest::artifact(index as u8 + 1, role, &bytes));
            }
        }
        let signer = |receipt: &SigningBindingReceipt, role, operation: Uuid| ContractSigner {
            account_id: receipt.statement.account().id().as_uuid().to_string(),
            account_role: role,
            binding_id: receipt.statement.material().binding().to_string(),
            binding_registered_at: receipt.registered_at.unix_timestamp(),
            operation_id: operation.to_string(),
        };
        encode(&ContractManifest {
            deployment_id: fields.deployment.to_string(),
            case_id: fields.case.as_uuid().to_string(),
            request_id: request.id.to_string(),
            document_id: fields.document.id.as_uuid().to_string(),
            document_version: u64::from(fields.document.version.get()),
            document_sha256: *fields.content_digest.as_bytes(),
            request_sha256: *request.digest().as_bytes(),
            expires_at: fields.expires_at_seconds,
            artifacts,
            signers: [
                signer(&self.client_binding, AccountRole::Client, first_operation),
                signer(
                    &self.firm_binding,
                    AccountRole::Litigator,
                    statement.operation(),
                ),
            ],
        })
        .unwrap()
    }
}
