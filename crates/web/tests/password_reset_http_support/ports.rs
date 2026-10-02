use std::io::Read;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

use application::identity::password_reset::{
    CompleteReset, IssueReset, PasswordResetDelivery, PasswordResetLimiter,
    PasswordResetRepository, ResetCandidate, ResetCompletion, ResetDeliveryOutcome, ResetEnvelope,
    ResetId, ResetIssueOutcome, ResetTokenSource,
};
use application::ApplicationError;
use domain::crypto::{DocumentHasher, PasswordHasher, PasswordVerification, Sha256Digest};
use domain::{identity::UserId, DomainError};
use uuid::Uuid;
use zeroize::Zeroizing;

pub const TOKEN: [u8; 32] = [0x5a; 32];

#[derive(Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Changed,
    Limited,
    Missing,
    Rejected,
    Uncertain,
    InvalidInput,
    HashError,
}

#[derive(Default)]
pub struct Ports {
    pub mode: Mutex<Mode>,
    calls: Mutex<Vec<&'static str>>,
    gate: Mutex<Option<Wait>>,
}

struct Wait {
    started: tokio::sync::oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}

pub struct Gate {
    started: tokio::sync::oneshot::Receiver<()>,
    release: Option<mpsc::Sender<()>>,
}

impl Gate {
    pub async fn entered(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), &mut self.started)
            .await
            .expect("completion never entered its blocking port")
            .unwrap();
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

impl Ports {
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
    pub fn count(&self, name: &str) -> usize {
        self.calls().iter().filter(|entry| **entry == name).count()
    }
    fn record(&self, name: &'static str) {
        self.calls.lock().unwrap().push(name);
    }
    pub fn block_inspect(&self) -> Gate {
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        *self.gate.lock().unwrap() = Some(Wait {
            started,
            release: wait,
        });
        Gate {
            started: ready,
            release: Some(release),
        }
    }
}

fn digest() -> Sha256Digest {
    Sha256Digest::from_array([0x39; 32])
}

impl PasswordResetRepository for Ports {
    fn issue(&self, _: IssueReset) -> Result<ResetIssueOutcome, ApplicationError> {
        panic!("request HTTP must submit admission instead of invoking completion service")
    }
    fn inspect(&self, value: Sha256Digest) -> Result<Option<ResetCandidate>, ApplicationError> {
        self.record("inspect");
        assert!(
            value == digest(),
            "completion used the wrong decoded capability"
        );
        let wait = self.gate.lock().unwrap().take();
        if let Some(wait) = wait {
            let _ = wait.started.send(());
            wait.release
                .recv_timeout(Duration::from_secs(4))
                .map_err(|_| ApplicationError::Port("private-reset-detail".into()))?;
        }
        match *self.mode.lock().unwrap() {
            Mode::Missing => Ok(None),
            Mode::InvalidInput => Err(ApplicationError::InvalidInput(
                "private-reset-detail".into(),
            )),
            _ => Ok(Some(ResetCandidate {
                id: ResetId::from_uuid(Uuid::from_u128(101)),
                user_id: UserId::from_uuid(Uuid::from_u128(41)),
                auth_generation: 7,
            })),
        }
    }
    fn complete(&self, command: CompleteReset) -> Result<ResetCompletion, ApplicationError> {
        self.record("complete");
        assert!(
            command.digest == digest(),
            "completion changed the capability digest"
        );
        assert_eq!(command.candidate.id.as_uuid(), Uuid::from_u128(101));
        assert!(
            command.password_hash == "controlled-hash",
            "HTTP bypassed password hashing"
        );
        match *self.mode.lock().unwrap() {
            Mode::Uncertain => Err(ApplicationError::Port("private-reset-detail".into())),
            Mode::Rejected => Ok(ResetCompletion::Rejected),
            _ => Ok(ResetCompletion::Changed),
        }
    }
    fn cancel_undelivered(&self, _: ResetId, _: Sha256Digest) -> Result<(), ApplicationError> {
        panic!("completion HTTP cannot cancel a delivery")
    }
}

impl PasswordResetLimiter for Ports {
    fn admit_request(&self, _: &str) -> Result<bool, ApplicationError> {
        panic!("request HTTP cannot await the application request limiter")
    }
    fn admit_completion(&self, _: Sha256Digest) -> Result<bool, ApplicationError> {
        self.record("limit");
        Ok(!matches!(*self.mode.lock().unwrap(), Mode::Limited))
    }
}

impl PasswordResetDelivery for Ports {
    fn deliver(&self, _: ResetEnvelope) -> Result<ResetDeliveryOutcome, ApplicationError> {
        panic!("completion HTTP cannot deliver email")
    }
}
impl ResetTokenSource for Ports {
    fn generate(&self) -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
        panic!("completion HTTP cannot generate a reset token")
    }
}
impl DocumentHasher for Ports {
    fn hash_bytes(&self, value: &[u8]) -> Sha256Digest {
        self.record("digest");
        let mut expected = Zeroizing::new(b"qadra:password-reset:v1\0".to_vec());
        expected.extend_from_slice(&TOKEN);
        assert!(
            value == expected.as_slice(),
            "HTTP failed to decode the exact canonical token"
        );
        digest()
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("completion must hash its fixed-size token bytes")
    }
}
impl PasswordHasher for Ports {
    fn hash(&self, password: &str) -> Result<String, DomainError> {
        self.record("hash");
        assert!(
            (12..=1024).contains(&password.len()),
            "invalid password reached the hasher"
        );
        if matches!(*self.mode.lock().unwrap(), Mode::HashError) {
            Err(DomainError::MalformedPasswordHash)
        } else {
            Ok("controlled-hash".into())
        }
    }
    fn verify(&self, _: &str, _: &str) -> Result<PasswordVerification, DomainError> {
        panic!("password reset cannot verify the forgotten password")
    }
}
