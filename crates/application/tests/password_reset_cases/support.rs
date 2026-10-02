use std::io::Read;
use std::sync::{Arc, Mutex};

use application::identity::password_reset::{
    CompleteReset, IssueReset, PasswordResetDelivery, PasswordResetLimiter, PasswordResetPorts,
    PasswordResetRepository, PasswordResetService, ResetCandidate, ResetCompletion,
    ResetDeliveryOutcome, ResetEnvelope, ResetId, ResetIssue, ResetIssueOutcome, ResetPolicy,
    ResetTokenSource,
};
use application::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, PasswordHasher, PasswordVerification, Sha256Digest};
use domain::identity::UserId;
use domain::DomainError;
use zeroize::Zeroizing;

pub const EMAIL: &str = "member@example.test";
pub const PASSWORD: &str = "new password for this account";
pub const TOKEN: [u8; 32] = [0x5a; 32];
pub const HASH: &str = "controlled-password-hash";

pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x39; 32])
}

pub fn policy() -> ResetPolicy {
    ResetPolicy::new(60, 2).unwrap()
}

pub fn deadline() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_800_000_060).unwrap()
}

pub fn issue_id(value: u128) -> ResetId {
    ResetId::from_uuid(uuid::Uuid::from_u128(value))
}

pub fn candidate() -> ResetCandidate {
    ResetCandidate {
        id: issue_id(101),
        user_id: UserId::from_uuid(uuid::Uuid::from_u128(41)),
        auth_generation: 7,
    }
}

pub struct Delivered {
    pub email: String,
    pub token: Vec<u8>,
    pub expires_at: OffsetDateTime,
}

pub struct State {
    pub calls: Vec<&'static str>,
    pub request_allowed: bool,
    pub completion_allowed: bool,
    pub ignore_issue: bool,
    pub issuance_id: ResetId,
    pub replace_issue_during_delivery: bool,
    pub candidate: Option<ResetCandidate>,
    pub complete_result: ResetCompletion,
    pub delivery_result: ResetDeliveryOutcome,
    pub fail_at: Option<&'static str>,
    pub hash_fails: bool,
    pub reject_after_hash: bool,
    pub requests: Vec<String>,
    pub completion_limits: Vec<Sha256Digest>,
    pub issues: Vec<IssueReset>,
    pub completions: Vec<CompleteReset>,
    pub cancellations: Vec<(ResetId, Sha256Digest)>,
    pub delivered: Vec<Delivered>,
    pub digest_inputs: Vec<Vec<u8>>,
    pub hashed_passwords: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            request_allowed: true,
            completion_allowed: true,
            ignore_issue: false,
            issuance_id: issue_id(101),
            replace_issue_during_delivery: false,
            candidate: Some(candidate()),
            complete_result: ResetCompletion::Changed,
            delivery_result: ResetDeliveryOutcome::Accepted,
            fail_at: None,
            hash_fails: false,
            reject_after_hash: false,
            requests: Vec::new(),
            completion_limits: Vec::new(),
            issues: Vec::new(),
            completions: Vec::new(),
            cancellations: Vec::new(),
            delivered: Vec::new(),
            digest_inputs: Vec::new(),
            hashed_passwords: Vec::new(),
        }
    }
}

impl State {
    fn called(&mut self, call: &'static str) -> Result<(), ApplicationError> {
        self.calls.push(call);
        if self.fail_at == Some(call) {
            Err(ApplicationError::Port(
                "controlled reset port failure".into(),
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct Doubles(pub Mutex<State>);

pub fn fixture() -> (PasswordResetService, Arc<Doubles>) {
    let ports = Arc::new(Doubles::default());
    let service = PasswordResetService::new(
        PasswordResetPorts {
            repository: ports.clone(),
            delivery: ports.clone(),
            limiter: ports.clone(),
            tokens: ports.clone(),
            digests: ports.clone(),
            passwords: ports.clone(),
        },
        policy(),
    );
    (service, ports)
}

impl PasswordResetRepository for Doubles {
    fn issue(&self, command: IssueReset) -> Result<ResetIssueOutcome, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("issue")?;
        state.issues.push(command);
        Ok(if state.ignore_issue {
            ResetIssueOutcome::Ignored
        } else {
            ResetIssueOutcome::Issued(ResetIssue {
                id: state.issuance_id,
                email: EMAIL.into(),
                expires_at: deadline(),
            })
        })
    }

    fn inspect(
        &self,
        token_digest: Sha256Digest,
    ) -> Result<Option<ResetCandidate>, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("inspect")?;
        assert_eq!(token_digest, digest());
        Ok(state.candidate.clone())
    }

    fn complete(&self, command: CompleteReset) -> Result<ResetCompletion, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("complete")?;
        state.completions.push(command);
        Ok(state.complete_result)
    }

    fn cancel_undelivered(
        &self,
        id: ResetId,
        token_digest: Sha256Digest,
    ) -> Result<(), ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("cancel")?;
        state.cancellations.push((id, token_digest));
        Ok(())
    }
}

impl PasswordResetDelivery for Doubles {
    fn deliver(&self, envelope: ResetEnvelope) -> Result<ResetDeliveryOutcome, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("delivery")?;
        state.delivered.push(Delivered {
            email: envelope.email,
            token: envelope.token.to_vec(),
            expires_at: envelope.expires_at,
        });
        if state.replace_issue_during_delivery {
            state.issuance_id = issue_id(202);
        }
        Ok(state.delivery_result)
    }
}

impl PasswordResetLimiter for Doubles {
    fn admit_request(&self, email: &str) -> Result<bool, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("request_limit")?;
        state.requests.push(email.into());
        Ok(state.request_allowed)
    }

    fn admit_completion(&self, token_digest: Sha256Digest) -> Result<bool, ApplicationError> {
        let mut state = self.0.lock().unwrap();
        state.called("completion_limit")?;
        state.completion_limits.push(token_digest);
        Ok(state.completion_allowed)
    }
}

impl ResetTokenSource for Doubles {
    fn generate(&self) -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
        self.0.lock().unwrap().called("token")?;
        Ok(Zeroizing::new(TOKEN))
    }
}

impl DocumentHasher for Doubles {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        let mut state = self.0.lock().unwrap();
        state.calls.push("digest");
        state.digest_inputs.push(bytes.to_vec());
        digest()
    }

    fn hash_stream(&self, _reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("reset digests must use the fixed token bytes")
    }
}

impl PasswordHasher for Doubles {
    fn hash(&self, password: &str) -> Result<String, DomainError> {
        let mut state = self.0.lock().unwrap();
        state.calls.push("hash");
        state.hashed_passwords.push(password.into());
        if state.hash_fails {
            return Err(DomainError::MalformedPasswordHash);
        }
        if state.reject_after_hash {
            state.complete_result = ResetCompletion::Rejected;
        }
        Ok(HASH.into())
    }

    fn verify(&self, _password: &str, _stored: &str) -> Result<PasswordVerification, DomainError> {
        panic!("password reset must not require the forgotten password")
    }
}

pub fn assert_port_failure(error: ApplicationError) {
    assert!(matches!(error, ApplicationError::Port(_)), "{error}");
}
