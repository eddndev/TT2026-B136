//! The evidence trust read authenticates first and never hides reader failures.
use application::credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot};
use application::evidence_trust::{
    CredentialTrustReader, EvidenceTrustService, EvidenceTrustWorkflow, ServedEvidenceMaterial,
};
use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult, SessionStatus,
};
use application::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::crypto::{CredentialTrustInspection, Sha256Digest};
use domain::identity::{Permission, Role, UserId};
use mockall::mock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

mock! {
    pub Identity {}
    impl IdentityWorkflow for Identity {
        fn bootstrap_owner(&self, email: &str, password: &str) -> Result<EnrollmentResult, ApplicationError>;
        fn create_user(&self, access_token: &str, email: &str, password: &str, role: Role) -> Result<EnrollmentResult, ApplicationError>;
        fn start_login(&self, email: &str, password: &str) -> Result<LoginChallenge, ApplicationError>;
        fn complete_totp(&self, token: &str, code: &str) -> Result<SessionResult, ApplicationError>;
        fn complete_recovery(&self, token: &str, code: &str) -> Result<SessionResult, ApplicationError>;
        fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError>;
        fn session_status(&self, token: &str) -> Result<SessionStatus, ApplicationError>;
        fn record_activity(&self, token: &str) -> Result<SessionStatus, ApplicationError>;
        fn authorize(&self, token: &str, permission: Permission) -> Result<Principal, ApplicationError>;
        fn logout(&self, token: &str) -> Result<(), ApplicationError>;
    }
}

enum Answer {
    Absent,
    Present,
    Failure,
}

struct Reader {
    calls: AtomicUsize,
    answer: Answer,
}

impl Reader {
    fn new(answer: Answer) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            answer,
        })
    }
}

impl CredentialTrustReader for Reader {
    fn current(&self) -> Result<Option<CredentialTrustSnapshot>, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.answer {
            Answer::Absent => Ok(None),
            Answer::Present => Ok(Some(snapshot())),
            Answer::Failure => Err(ApplicationError::Port("trust store unavailable".into())),
        }
    }
}

fn snapshot() -> CredentialTrustSnapshot {
    CredentialTrustSnapshot {
        deployment_id: Uuid::from_u128(7),
        revision: CredentialTrustRevision::initial(),
        inspection: CredentialTrustInspection {
            root_der: vec![1, 2, 3],
            crl_der: vec![4, 5],
            root_fingerprint: Sha256Digest::from_array([0x11; 32]),
            crl_digest: Sha256Digest::from_array([0x22; 32]),
            crl_number: 3,
            crl_this_update: 10,
            crl_next_update: 20,
            valid_from: 10,
            valid_until: 20,
        },
        published_at: OffsetDateTime::from_unix_timestamp(15).unwrap(),
        published_by: "publisher".into(),
    }
}

fn material() -> Arc<ServedEvidenceMaterial> {
    Arc::new(ServedEvidenceMaterial {
        seal_root_der: vec![9, 8, 7],
        seal_root_fingerprint: "ab".repeat(32),
        seal_crl_der: vec![6, 5],
        seal_crl_digest: "cd".repeat(32),
        time_stamping: vec![(vec![4, 4], "ef".repeat(32))],
    })
}

fn identity(accepted: bool) -> Arc<MockIdentity> {
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(1)
        .returning(move |_| {
            if accepted {
                Ok(Principal {
                    id: UserId::new(),
                    email: "client@example.test".into(),
                    role: Role::Client,
                })
            } else {
                Err(ApplicationError::InvalidSession)
            }
        });
    Arc::new(identity)
}

#[test]
fn a_rejected_token_never_reads_credential_trust() {
    let reader = Reader::new(Answer::Present);
    let service = EvidenceTrustService::new(identity(false), Some(reader.clone()), material());

    let result = service.current("session");

    assert!(matches!(result, Err(ApplicationError::InvalidSession)));
    assert_eq!(reader.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn without_a_reader_individual_signing_is_absent_and_material_is_served() {
    let service = EvidenceTrustService::new(identity(true), None, material());

    let trust = service.current("session").unwrap();

    assert!(trust.individual_signing.is_none());
    assert_eq!(*trust.material, *material());
}

#[test]
fn an_absent_snapshot_is_not_an_error() {
    let reader = Reader::new(Answer::Absent);
    let service = EvidenceTrustService::new(identity(true), Some(reader.clone()), material());

    let trust = service.current("session").unwrap();

    assert!(trust.individual_signing.is_none());
    assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn the_current_snapshot_is_returned_with_the_material() {
    let reader = Reader::new(Answer::Present);
    let service = EvidenceTrustService::new(identity(true), Some(reader.clone()), material());

    let trust = service.current("session").unwrap();

    assert_eq!(trust.individual_signing, Some(snapshot()));
    assert_eq!(trust.material.time_stamping.len(), 1);
}

#[test]
fn a_reader_error_propagates() {
    let reader = Reader::new(Answer::Failure);
    let service = EvidenceTrustService::new(identity(true), Some(reader.clone()), material());

    let result = service.current("session");

    assert!(matches!(result, Err(ApplicationError::Port(_))));
    assert_eq!(reader.calls.load(Ordering::SeqCst), 1);
}
