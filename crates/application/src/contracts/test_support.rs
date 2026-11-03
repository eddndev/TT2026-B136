//! Structural values for the pure contract rules; no cryptography is
//! checked by the code these values exercise.

use domain::document_signing::Uuid;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{
        CertificateSummary, CredentialCertificate, CredentialCheck, CredentialTrustInspection,
        DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest, Signature,
    },
    document_signing::{
        contract::{ContractRequest, ContractRequestFields},
        SignerAccount, SigningBindingMaterial, SigningBindingStatement,
    },
    identity::{Role, UserId},
};

use super::model::{ContractEvent, ContractRoot, ContractSnapshot, EventKind, RequestRecord};
use crate::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    document_signing::SigningBindingReceipt,
    identity::Principal,
};

pub const CLIENT: u128 = 10;
pub const FIRM: u128 = 20;
pub const REQUESTER: u128 = 30;
pub const OTHER: u128 = 40;

pub fn user(n: u128) -> UserId {
    UserId::from_uuid(Uuid::from_u128(n))
}

pub fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

pub fn principal(n: u128, role: Role) -> Principal {
    Principal {
        id: user(n),
        email: format!("{n}@example.test"),
        role,
    }
}

fn trust() -> CredentialTrustSnapshot {
    CredentialTrustSnapshot {
        deployment_id: Uuid::from_u128(1),
        revision: CredentialTrustRevision::new(1).unwrap(),
        inspection: CredentialTrustInspection {
            root_der: b"root".to_vec(),
            crl_der: b"crl".to_vec(),
            root_fingerprint: Sha256Digest::from_array([7; 32]),
            crl_digest: Sha256Digest::from_array([8; 32]),
            crl_number: 1,
            crl_this_update: 0,
            crl_next_update: 10_000_000,
            valid_from: 0,
            valid_until: 10_000_000,
        },
        published_at: at(0),
        published_by: "admin".into(),
    }
}

pub fn binding(account: u128, leaf: u8) -> SigningBindingReceipt {
    let trust = trust();
    let signer = SignerAccount::from_signed_counters(user(account), 1, 1).unwrap();
    let material = SigningBindingMaterial::new(
        trust.deployment_id,
        Uuid::from_u128(account + 1000),
        trust.inspection.root_fingerprint,
        Sha256Digest::from_array([leaf; 32]),
        1,
    )
    .unwrap();
    let statement = SigningBindingStatement::new(signer, user(account), material).unwrap();
    SigningBindingReceipt {
        statement,
        check: CredentialCheck {
            certificate: CredentialCertificate {
                der: vec![leaf],
                fingerprint: Sha256Digest::from_array([leaf; 32]),
                summary: CertificateSummary {
                    subject: "Signer".into(),
                    issuer: "Root".into(),
                    serial_hex: "01".into(),
                    not_before_unix: 0,
                    not_after_unix: 10_000_000,
                },
            },
            trust: trust.inspection.clone(),
            statement_digest: Sha256Digest::from_array([9; 32]),
            signature: Signature::from_bytes(vec![1; 384]).unwrap(),
            checked_at: 0,
            valid_from: 0,
            valid_until: 10_000_000,
        },
        trust,
        registered_at: at(0),
        withdrawn_at: None,
    }
}

/// A request created at `created` that expires one day later.
pub fn request(id: u128, created: i64) -> RequestRecord {
    let fields = ContractRequestFields {
        deployment: Uuid::from_u128(1),
        request: Uuid::from_u128(id),
        case: CaseId::from_uuid(Uuid::from_u128(2)),
        document: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(3)),
            version: DocumentVersion::new(1).unwrap(),
        },
        content_digest: Sha256Digest::from_array([4; 32]),
        client: user(CLIENT),
        firm_signer: user(FIRM),
        requester: user(REQUESTER),
        client_leaf: Sha256Digest::from_array([5; 32]),
        firm_leaf: Sha256Digest::from_array([6; 32]),
        created_at_seconds: created,
        expires_at_seconds: created + 86_400,
    };
    RequestRecord {
        request: ContractRequest::new(fields).unwrap(),
        requester_email: "30@example.test".into(),
        ceremony_reference: "HF-1".into(),
        subject: None,
        created_at: at(created),
        client_binding: binding(CLIENT, 5),
        firm_binding: binding(FIRM, 6),
        client_role: Role::Client,
        firm_role: Role::Litigator,
        signatures: Vec::new(),
        completion: None,
    }
}

pub fn event(
    revision: u32,
    kind: EventKind,
    request: Option<u128>,
    at_seconds: i64,
) -> ContractEvent {
    ContractEvent {
        revision,
        kind,
        request: request.map(Uuid::from_u128),
        actor: Some(user(REQUESTER)),
        actor_label: "30@example.test".into(),
        reason: None,
        at: at(at_seconds),
        operation: Uuid::from_u128(u128::from(revision) + 500),
    }
}

/// A contract whose events follow `kinds`, all naming request 100 except
/// `created` and `abandoned`.
pub fn snapshot(kinds: &[EventKind], requests: Vec<RequestRecord>) -> ContractSnapshot {
    let events = kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            let request =
                (!matches!(kind, EventKind::Created | EventKind::Abandoned)).then_some(100);
            event(index as u32 + 1, *kind, request, 0)
        })
        .collect();
    ContractSnapshot {
        root: ContractRoot {
            id: Uuid::from_u128(50),
            case: CaseId::from_uuid(Uuid::from_u128(2)),
            document: DocumentId::from_uuid(Uuid::from_u128(3)),
            title: "Servicios".into(),
            created_by: user(REQUESTER),
            created_by_email: "30@example.test".into(),
            created_at: at(0),
        },
        events,
        requests,
        case_scope: false,
        latest_version: 1,
        case_closed: false,
    }
}
