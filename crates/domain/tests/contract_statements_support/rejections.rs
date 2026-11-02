//! Structural rejections of the contract request and party statements, and
//! their separation from every other signed statement of the domain.

use super::support::{client_statement, fields, firm_statement, request, user, Party};
use domain::crypto::Sha256Digest;
use domain::document_signing::contract::{
    ContractPartyStatement, ContractPosition, ContractRequest, ContractStatementError as E,
};
use domain::document_signing::{
    DocumentConsent, SignerAccount, SigningBindingStatement, SigningError, Uuid,
};
use domain::identity::Role;

fn request_error(bytes: &[u8]) -> E {
    ContractRequest::from_canonical_bytes(bytes).unwrap_err()
}

fn statement_error(bytes: &[u8]) -> E {
    ContractPartyStatement::from_canonical_bytes(bytes).unwrap_err()
}

fn changed<const N: usize>(
    mut bytes: [u8; N],
    range: std::ops::Range<usize>,
    value: u8,
) -> [u8; N] {
    bytes[range].fill(value);
    bytes
}

#[test]
fn wrong_lengths_magic_and_format_bytes_are_rejected() {
    let request = request().canonical_bytes();
    let statement = firm_statement().canonical_bytes();
    assert_eq!(request_error(&request[..239]), E::StatementMismatch);
    assert_eq!(
        request_error(&[&request[..], &[0]].concat()),
        E::StatementMismatch
    );
    assert_eq!(statement_error(&statement[..302]), E::StatementMismatch);
    assert_eq!(
        statement_error(&[&statement[..], &[0]].concat()),
        E::StatementMismatch
    );
    assert_eq!(request_error(&[]), E::StatementMismatch);
    assert_eq!(statement_error(&[]), E::StatementMismatch);
    for position in [0, 7, 8, 9] {
        let mut invalid = request;
        invalid[position] ^= 1;
        assert_eq!(request_error(&invalid), E::StatementMismatch, "{position}");
        let mut invalid = statement;
        invalid[position] ^= 1;
        assert_eq!(
            statement_error(&invalid),
            E::StatementMismatch,
            "{position}"
        );
    }
}

#[test]
fn every_other_signed_statement_is_rejected_by_both_decoders() {
    let others: Vec<Vec<u8>> = vec![
        binding().canonical_bytes().to_vec(),
        magic_only(b"OWNCERT1", 150),
        magic_only(b"OWNAUTH1", 182),
        consent().canonical_bytes().to_vec(),
        retagged(&firm_statement().canonical_bytes(), b"CTRTREQ1"),
        retagged(&request().canonical_bytes(), b"CTRTSIG1"),
    ];
    for bytes in &others {
        assert!(ContractRequest::from_canonical_bytes(bytes).is_err());
        assert!(ContractPartyStatement::from_canonical_bytes(bytes).is_err());
    }
    assert!(ContractRequest::from_canonical_bytes(&firm_statement().canonical_bytes()).is_err());
    assert!(ContractPartyStatement::from_canonical_bytes(&request().canonical_bytes()).is_err());
    for bytes in [
        request().canonical_bytes().to_vec(),
        client_statement().canonical_bytes().to_vec(),
    ] {
        assert!(SigningBindingStatement::from_canonical_bytes(&bytes).is_err());
        assert!(DocumentConsent::from_canonical_bytes(&bytes).is_err());
    }
}

#[test]
fn request_fields_reapply_identity_party_and_validity_rules() {
    let bytes = request().canonical_bytes();
    for range in [10..26, 26..42, 42..58, 58..74, 110..126, 126..142, 142..158] {
        assert_eq!(request_error(&changed(bytes, range, 0)), E::InvalidIdentity);
    }
    assert_eq!(
        request_error(&changed(bytes, 74..78, 0)),
        E::StatementMismatch
    );
    for (offset, value) in [(238, 1), (238, 3), (239, 0), (239, 2)] {
        assert_eq!(
            request_error(&changed(bytes, offset..offset + 1, value)),
            E::StatementMismatch
        );
    }
    let mut same = bytes;
    same.copy_within(110..126, 126);
    assert_eq!(request_error(&same), E::SameParty);
    let mut same = bytes;
    same.copy_within(110..126, 142);
    assert_eq!(request_error(&same), E::SameParty);
    let mut same = bytes;
    same.copy_within(158..190, 190);
    assert_eq!(request_error(&same), E::SameCertificate);
    let mut late = bytes;
    late.copy_within(230..238, 222);
    assert_eq!(request_error(&late), E::InvalidValidity);
    let mut owner_requests = fields();
    owner_requests.requester = owner_requests.firm_signer;
    assert!(ContractRequest::new(owner_requests).is_ok());
    let mut reversed = fields();
    reversed.created_at_seconds = reversed.expires_at_seconds + 1;
    assert_eq!(ContractRequest::new(reversed), Err(E::InvalidValidity));
}

#[test]
fn party_statement_fields_reapply_prefix_and_position_rules() {
    let firm = firm_statement().canonical_bytes();
    let client = client_statement().canonical_bytes();
    let signing = |error| E::Signing(error);
    for range in [10..26, 62..78, 94..110] {
        assert_eq!(
            statement_error(&changed(firm, range, 0)),
            signing(SigningError::InvalidIdentity)
        );
    }
    assert_eq!(
        statement_error(&changed(firm, 58..62, 0)),
        signing(SigningError::InvalidTrustRevision)
    );
    assert_eq!(
        statement_error(&changed(firm, 78..86, 0xff)),
        signing(SigningError::InvalidAccountVersion)
    );
    for range in [142..158, 158..174] {
        assert_eq!(
            statement_error(&changed(firm, range, 0)),
            E::InvalidIdentity
        );
    }
    for value in [0, 3, 0xff] {
        assert_eq!(
            statement_error(&changed(firm, 238..239, value)),
            E::InvalidPosition
        );
    }
    for range in [239..271, 271..303] {
        assert_eq!(
            statement_error(&changed(firm, range, 0)),
            E::InvalidPosition
        );
    }
    assert_eq!(
        statement_error(&changed(firm, 238..239, 1)),
        E::StatementMismatch
    );
    assert_eq!(
        statement_error(&changed(client, 302..303, 1)),
        E::StatementMismatch
    );
    assert_eq!(
        statement_error(&changed(client, 238..239, 2)),
        E::InvalidPosition
    );
}

#[test]
fn party_statement_constructor_rejects_nil_references_and_empty_chain_digests() {
    let party = Party::firm();
    let zero = Sha256Digest::from_array([0; 32]);
    let build = |operation: Uuid, request: Uuid, position| {
        ContractPartyStatement::new(
            party.account(),
            party.material(),
            operation,
            request,
            zero,
            zero,
            position,
        )
    };
    let id = Uuid::from_bytes([7; 16]);
    assert_eq!(
        build(Uuid::nil(), id, ContractPosition::Client),
        Err(E::InvalidIdentity)
    );
    assert_eq!(
        build(id, Uuid::nil(), ContractPosition::Client),
        Err(E::InvalidIdentity)
    );
    let empty = ContractPosition::Firm {
        signature_digest: zero,
        token_digest: Sha256Digest::from_array([1; 32]),
    };
    assert_eq!(build(id, id, empty), Err(E::InvalidPosition));
    assert!(build(id, id, ContractPosition::Client).is_ok());
}

fn magic_only(magic: &[u8; 8], length: usize) -> Vec<u8> {
    let mut bytes = vec![0x5a; length];
    bytes[..8].copy_from_slice(magic);
    bytes[8] = 1;
    bytes[9] = 1;
    bytes
}

fn retagged(bytes: &[u8], magic: &[u8; 8]) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    bytes[..8].copy_from_slice(magic);
    bytes
}

fn litigator() -> SignerAccount {
    SignerAccount::new(user(0x33), Role::Litigator, true, 9, 4).unwrap()
}

fn binding() -> SigningBindingStatement {
    SigningBindingStatement::new(litigator(), user(0x33), Party::client().material()).unwrap()
}

fn consent() -> DocumentConsent {
    let fields = fields();
    DocumentConsent::new(
        litigator(),
        Party::client().material(),
        Uuid::from_bytes([0x66; 16]),
        fields.case,
        fields.document,
        fields.content_digest,
        Sha256Digest::from_array([0x99; 32]),
    )
    .unwrap()
}
