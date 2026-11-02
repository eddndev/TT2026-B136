//! Values of the shared contract statement vector, read field by field.

use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::document_signing::contract::{
    ContractPartyStatement, ContractPosition, ContractRequest, ContractRequestFields,
};
use domain::document_signing::{SignerAccount, SigningBindingMaterial, Uuid};
use domain::identity::{Role, UserId};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn vector() -> Value {
    serde_json::from_str(include_str!("../fixtures/contract_statement_vectors.json")).unwrap()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

/// The `hex` bytes of one encoded object of the vector.
pub fn bytes(encoded: &Value) -> Vec<u8> {
    unhex(encoded["hex"].as_str().unwrap())
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn user(byte: u8) -> UserId {
    UserId::from_uuid(Uuid::from_bytes([byte; 16]))
}

fn text<'a>(object: &'a Value, key: &str) -> &'a str {
    object[key].as_str().unwrap()
}

fn uuid(object: &Value, key: &str) -> Uuid {
    Uuid::parse_str(text(object, key)).unwrap()
}

fn digest(object: &Value, key: &str) -> Sha256Digest {
    Sha256Digest::from_hex(text(object, key)).unwrap()
}

fn integer(object: &Value, key: &str) -> u64 {
    object[key].as_u64().unwrap()
}

/// The typed request fields of the vector.
pub fn fields() -> ContractRequestFields {
    let inputs = &vector()["request_inputs"];
    ContractRequestFields {
        deployment: uuid(inputs, "deployment_id"),
        request: uuid(inputs, "request_id"),
        case: CaseId::from_uuid(uuid(inputs, "case_id")),
        document: DocumentVersionRef {
            id: DocumentId::from_uuid(uuid(inputs, "document_id")),
            version: DocumentVersion::new(integer(inputs, "document_version") as u32).unwrap(),
        },
        content_digest: digest(inputs, "document_sha256"),
        client: UserId::from_uuid(uuid(inputs, "client_account_id")),
        firm_signer: UserId::from_uuid(uuid(inputs, "firm_account_id")),
        requester: UserId::from_uuid(uuid(inputs, "requester_account_id")),
        client_leaf: digest(inputs, "client_certificate_sha256"),
        firm_leaf: digest(inputs, "firm_certificate_sha256"),
        created_at_seconds: inputs["created_at_seconds"].as_i64().unwrap(),
        expires_at_seconds: inputs["expires_at_seconds"].as_i64().unwrap(),
    }
}

pub fn request() -> ContractRequest {
    ContractRequest::new(fields()).unwrap()
}

/// One party object of the vector with its role.
pub struct Party {
    values: Value,
    role: Role,
}

impl Party {
    pub fn client() -> Self {
        Self {
            values: vector()["client_party"].clone(),
            role: Role::Client,
        }
    }

    pub fn firm() -> Self {
        Self {
            values: vector()["firm_party"].clone(),
            role: Role::Owner,
        }
    }

    pub fn bytes(&self, key: &str) -> Vec<u8> {
        unhex(text(&self.values, key))
    }

    pub fn account(&self) -> SignerAccount {
        SignerAccount::for_contract(
            UserId::from_uuid(uuid(&self.values, "account_id")),
            self.role,
            true,
            integer(&self.values, "account_revision"),
            integer(&self.values, "authentication_generation"),
        )
        .unwrap()
    }

    pub fn material(&self) -> SigningBindingMaterial {
        let inputs = &vector()["request_inputs"];
        SigningBindingMaterial::new(
            uuid(inputs, "deployment_id"),
            uuid(&self.values, "binding_id"),
            digest(&self.values, "root_sha256"),
            digest(&self.values, "certificate_sha256"),
            integer(&self.values, "trust_revision") as u32,
        )
        .unwrap()
    }

    pub fn position(&self) -> ContractPosition {
        match integer(&self.values, "position") {
            1 => ContractPosition::Client,
            2 => ContractPosition::Firm {
                signature_digest: digest(&self.values, "prior_signature_sha256"),
                token_digest: digest(&self.values, "prior_token_sha256"),
            },
            _ => panic!("unknown position"),
        }
    }

    pub fn statement(&self) -> ContractPartyStatement {
        let request = request();
        ContractPartyStatement::new(
            self.account(),
            self.material(),
            uuid(&self.values, "operation_id"),
            request.fields().request,
            Sha256Digest::from_array(sha256(&request.canonical_bytes())),
            request.fields().content_digest,
            self.position(),
        )
        .unwrap()
    }
}

pub fn client_statement() -> ContractPartyStatement {
    Party::client().statement()
}

pub fn firm_statement() -> ContractPartyStatement {
    Party::firm().statement()
}
