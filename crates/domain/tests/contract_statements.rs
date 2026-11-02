//! Request and party statements of bilateral contract signing against the
//! shared vector `fixtures/contract_statement_vectors.json`, produced by the
//! independent generator beside it. Layouts:
//! `docs/adr/0086-bilateral-contract-signing.md`.

#[path = "contract_statements_support/rejections.rs"]
mod rejections;
#[path = "contract_statements_support/mod.rs"]
mod support;

use domain::document_signing::contract::{
    ContractPartyStatement, ContractRequest, PARTY_STATEMENT_LEN, REQUEST_LEN,
};
use domain::document_signing::{SignerAccount, SigningError};
use domain::identity::Role;
use support::{bytes, client_statement, firm_statement, hex, request, sha256, user, vector, Party};

#[test]
fn lengths_are_the_frozen_layout_lengths() {
    assert_eq!(REQUEST_LEN, 240);
    assert_eq!(PARTY_STATEMENT_LEN, 303);
}

#[test]
fn request_encodes_and_decodes_the_shared_vector() {
    let expected = &vector()["request"];
    let encoded = request().canonical_bytes();
    assert_eq!(hex(&encoded), expected["hex"].as_str().unwrap());
    assert_eq!(encoded.len() as u64, expected["length"].as_u64().unwrap());
    assert_eq!(hex(&sha256(&encoded)), expected["sha256"].as_str().unwrap());
    let decoded = ContractRequest::from_canonical_bytes(&bytes(expected)).unwrap();
    assert_eq!(decoded, request());
    assert!(decoded.require_exact_bytes(&encoded).is_ok());
}

#[test]
fn both_party_statements_encode_and_decode_the_shared_vector() {
    for (name, statement) in [
        ("client_statement", client_statement()),
        ("firm_statement", firm_statement()),
    ] {
        let expected = &vector()[name];
        let encoded = statement.canonical_bytes();
        assert_eq!(hex(&encoded), expected["hex"].as_str().unwrap(), "{name}");
        assert_eq!(encoded.len() as u64, expected["length"].as_u64().unwrap());
        assert_eq!(hex(&sha256(&encoded)), expected["sha256"].as_str().unwrap());
        let decoded = ContractPartyStatement::from_canonical_bytes(&bytes(expected)).unwrap();
        assert_eq!(decoded, statement, "{name}");
        assert!(decoded.require_exact_bytes(&encoded).is_ok());
    }
}

#[test]
fn party_statements_chain_to_the_request_and_the_first_signature() {
    let request_digest = sha256(&request().canonical_bytes());
    let client = client_statement();
    let firm = firm_statement();
    for statement in [client, firm] {
        assert_eq!(statement.request_digest().as_bytes(), &request_digest);
        assert_eq!(statement.request(), request().fields().request);
        assert_eq!(
            statement.content_digest(),
            request().fields().content_digest
        );
    }
    assert_eq!(client.position().number(), 1);
    assert_eq!(client.position().signature_digest(), None);
    assert_eq!(firm.position().number(), 2);
    let party = Party::firm();
    let signature = sha256(&party.bytes("prior_signature_hex"));
    let token = sha256(&party.bytes("prior_token_hex"));
    assert_eq!(
        firm.position().signature_digest().unwrap().as_bytes(),
        &signature
    );
    assert_eq!(firm.position().token_digest().unwrap().as_bytes(), &token);
    let encoded = client.canonical_bytes();
    assert_eq!(&encoded[239..], &[0; 64]);
}

#[test]
fn contract_signers_admit_owner_litigator_and_client_only_when_active() {
    for role in [Role::Owner, Role::Litigator, Role::Client] {
        assert!(SignerAccount::for_contract(user(1), role, true, 3, 1).is_ok());
        assert_eq!(
            SignerAccount::for_contract(user(1), role, false, 3, 1),
            Err(SigningError::ContractSignerRequired)
        );
    }
    assert_eq!(
        SignerAccount::for_contract(user(1), Role::Paralegal, true, 3, 1),
        Err(SigningError::ContractSignerRequired)
    );
    assert_eq!(
        SignerAccount::for_contract(user(0), Role::Client, true, 3, 1),
        Err(SigningError::InvalidIdentity)
    );
    assert_eq!(
        SignerAccount::for_contract(user(1), Role::Client, true, 3, 4),
        Err(SigningError::InvalidAccountVersion)
    );
    assert_eq!(
        SignerAccount::new(user(1), Role::Client, true, 3, 1),
        Err(SigningError::LitigatorRequired)
    );
}
