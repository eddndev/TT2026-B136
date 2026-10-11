use application::identity::invitations::InvitationService;
pub use application::identity::{
    invitations::*, password_reset::ResetTokenSource, EnrollmentResult, IdentityWorkflow,
    LoginChallenge, Principal, SecretProtector, SessionResult, SessionStatus, UserRecord,
    UserRepository,
};
pub use application::ApplicationError;
pub use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{
        DocumentHasher, PasswordHasher, PasswordVerification, RecoveryCodeGenerator,
        RecoveryCodeSet, Sha256Digest, TotpEnrollment, TotpProvider, TotpVerification,
        RECOVERY_CODE_COUNT,
    },
    identity::{Permission, Role, UserId},
    DomainError,
};
pub use std::{
    io::Read,
    sync::{Arc, Mutex},
};
pub use zeroize::Zeroizing;

#[path = "mocks.rs"]
mod mocks;
pub use mocks::*;
#[path = "crypto.rs"]
mod crypto;
#[path = "store.rs"]
mod store;

pub const EMAIL: &str = "recipient@example.test";
pub const PASSWORD: &str = "the chosen recipient password";
pub const RAW_TOKEN: [u8; 32] = [0x5a; 32];
pub const CODE: &str = "123456";

pub fn password_hash(value: &str) -> String {
    let encoded: String = value.bytes().map(|byte| format!("{byte:02x}")).collect();
    format!("fake:{encoded}")
}
pub fn operation(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub fn id() -> InvitationId {
    InvitationId::from_uuid(operation(10))
}
pub fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
}
pub fn expiry() -> OffsetDateTime {
    now() + time::Duration::seconds(INVITATION_TTL_SECONDS)
}
pub fn owner_record() -> UserRecord {
    UserRecord {
        id: UserId::from_uuid(operation(1)),
        email: "owner@example.test".into(),
        password_hash: "fake:owner password".into(),
        role: Role::Owner,
        active: true,
        protected_totp_secret: vec![1],
        recovery_codes: RecoveryCodeSet::from_hashes(vec!["fake:code".into(); RECOVERY_CODE_COUNT])
            .unwrap(),
        revision: 7,
        auth_generation: 4,
    }
}
pub fn actor() -> InvitationActor {
    InvitationActor {
        principal: Principal::from(&owner_record()),
        revision: 7,
        auth_generation: 4,
    }
}
pub fn candidate() -> InvitationCandidate {
    InvitationCandidate {
        id: id(),
        revision: 1,
        email: EMAIL.into(),
        role: Role::Litigator,
        expires_at: expiry(),
        pending: None,
    }
}
pub fn summary() -> InvitationSummary {
    InvitationSummary {
        id: id(),
        revision: 1,
        operation_id: operation(10),
        email: EMAIL.into(),
        role: Role::Litigator,
        status: InvitationStatus::Pending,
        delivery: InvitationDeliveryState::Pending,
        issued_at: now(),
        expires_at: expiry(),
        accepted_at: None,
    }
}
pub fn issue_request() -> InvitationIssueRequest {
    InvitationIssueRequest {
        operation_id: operation(10),
        email: EMAIL.into(),
        role: Role::Litigator,
    }
}
pub fn resend_request() -> InvitationResendRequest {
    InvitationResendRequest {
        operation_id: operation(11),
        expected_revision: 1,
    }
}
pub fn query() -> InvitationQuery {
    InvitationQuery {
        after: None,
        limit: 20,
    }
}

pub struct SealedValue {
    pub context: InvitationProtectionContext,
    pub plaintext: Vec<u8>,
    pub opaque: Vec<u8>,
}
pub struct State {
    pub calls: Vec<&'static str>,
    pub principal: Option<Principal>,
    pub owner: UserRecord,
    pub candidate: Option<InvitationCandidate>,
    pub user: Option<UserRecord>,
    pub completion: Option<InvitationCompletion>,
    pub sealed: Vec<SealedValue>,
    pub digest_inputs: Vec<Vec<u8>>,
    pub hashes: Vec<String>,
    pub tokens: usize,
    pub token_value: [u8; 32],
    pub enrollments: usize,
    pub recoveries: usize,
    pub pending_writes: usize,
    pub finish_commands: Vec<FinishInvitation>,
    pub current_time: OffsetDateTime,
    pub limited: bool,
    pub change_owner_on_protect: bool,
    pub finish_error: bool,
    pub lose_finish_response: bool,
    pub corrupt_ciphertext: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            calls: vec![],
            principal: Some(actor().principal),
            owner: owner_record(),
            candidate: Some(candidate()),
            user: None,
            completion: None,
            sealed: vec![],
            digest_inputs: vec![],
            hashes: vec![],
            tokens: 0,
            token_value: RAW_TOKEN,
            enrollments: 0,
            recoveries: 0,
            pending_writes: 0,
            finish_commands: vec![],
            current_time: now(),
            limited: false,
            change_owner_on_protect: false,
            finish_error: false,
            lose_finish_response: false,
            corrupt_ciphertext: false,
        }
    }
}
pub struct Harness {
    pub state: Arc<Mutex<State>>,
    pub identity: MockIdentity,
    pub users: MockUsers,
    pub store: MockStore,
    pub protector: MockProtector,
    pub secrets: MockSecrets,
    pub passwords: MockPasswords,
    pub totp: MockTotp,
    pub recovery: MockRecovery,
    pub tokens: MockTokens,
    pub limiter: MockLimiter,
    pub digests: MockDigests,
    pub clock: MockTime,
}
impl Harness {
    pub fn new() -> Self {
        let mut value = Self {
            state: Arc::new(Mutex::new(State::default())),
            identity: MockIdentity::new(),
            users: MockUsers::new(),
            store: MockStore::new(),
            protector: MockProtector::new(),
            secrets: MockSecrets::new(),
            passwords: MockPasswords::new(),
            totp: MockTotp::new(),
            recovery: MockRecovery::new(),
            tokens: MockTokens::new(),
            limiter: MockLimiter::new(),
            digests: MockDigests::new(),
            clock: MockTime::new(),
        };
        let state = value.state.clone();
        value
            .identity
            .expect_authenticate()
            .returning(move |token| {
                assert_eq!(token, "session");
                let mut state = state.lock().unwrap();
                state.calls.push("authenticate");
                state
                    .principal
                    .clone()
                    .ok_or(ApplicationError::InvalidSession)
            });
        let state = value.state.clone();
        value.users.expect_find_by_id().returning(move |id| {
            let mut state = state.lock().unwrap();
            state.calls.push("find_user");
            if id == state.owner.id {
                return Ok(Some(state.owner.clone()));
            }
            Ok(state.user.as_ref().filter(|user| user.id == id).cloned())
        });
        let state = value.state.clone();
        value.users.expect_find_by_email().returning(move |email| {
            let mut state = state.lock().unwrap();
            state.calls.push("find_email");
            Ok(state
                .user
                .as_ref()
                .filter(|user| user.email == email)
                .cloned())
        });
        crypto::configure(&mut value);
        store::configure(&mut value);
        value
    }
    pub fn service(self) -> InvitationService {
        InvitationService::new(InvitationPorts {
            identity: Arc::new(self.identity),
            users: Arc::new(self.users),
            store: Arc::new(self.store),
            protector: Arc::new(self.protector),
            secrets: Arc::new(self.secrets),
            passwords: Arc::new(self.passwords),
            totp: Arc::new(self.totp),
            recovery: Arc::new(self.recovery),
            tokens: Arc::new(self.tokens),
            limiter: Arc::new(self.limiter),
            digests: Arc::new(self.digests),
            clock: Arc::new(self.clock),
        })
    }
}
pub fn prepare(service: &InvitationService) -> InvitationEnrollment {
    service
        .prepare(&RAW_TOKEN, operation(20), PASSWORD)
        .unwrap()
}
pub fn token_digest(state: &State) -> Sha256Digest {
    let expected = [b"qadra:invitation:v1\0".as_slice(), RAW_TOKEN.as_slice()].concat();
    let index = state
        .digest_inputs
        .iter()
        .position(|value| *value == expected)
        .unwrap();
    Sha256Digest::from_array([(index + 1) as u8; 32])
}
