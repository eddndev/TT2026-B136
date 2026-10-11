use std::sync::{mpsc, Mutex};
use std::time::Duration;

use application::{identity::invitations::*, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use zeroize::Zeroizing;

use super::{invitation, operation, CODE, PASSWORD, PRIVATE_ERROR, REVISION, SECRET, TOKEN};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Issue(String, InvitationIssueRequest),
    Resend(String, InvitationId, InvitationResendRequest),
    List(String, InvitationQuery),
    Get(String, InvitationId),
    Recipient(&'static str),
}
#[derive(Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Ready,
    MissingReceipt,
    Rejected,
    MfaRejected,
    Conflict,
    InvalidInput,
    Uncertain,
}
#[derive(Default)]
pub struct Workflow {
    pub mode: Mutex<Mode>,
    calls: Mutex<Vec<Call>>,
    gate: Mutex<Option<Wait>>,
}
struct Wait {
    entered: tokio::sync::oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}
pub struct Gate {
    entered: tokio::sync::oneshot::Receiver<()>,
    release: Option<mpsc::Sender<()>>,
}
impl Gate {
    pub async fn entered(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), &mut self.entered)
            .await
            .expect("invitation workflow did not start")
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
impl Workflow {
    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
    pub fn block(&self) -> Gate {
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        *self.gate.lock().unwrap() = Some(Wait {
            entered,
            release: wait,
        });
        Gate {
            entered: ready,
            release: Some(release),
        }
    }
    fn owner(&self, token: &str, call: Call) -> Result<(), ApplicationError> {
        self.calls.lock().unwrap().push(call);
        if token == "owner" {
            Ok(())
        } else {
            Err(ApplicationError::PermissionDenied)
        }
    }
    fn recipient(
        &self,
        name: &'static str,
        token: &[u8],
        id: Uuid,
        password: &str,
    ) -> Result<(), ApplicationError> {
        assert_eq!(token, TOKEN);
        assert_eq!(id, operation());
        assert_eq!(password, PASSWORD);
        self.calls.lock().unwrap().push(Call::Recipient(name));
        let wait = self.gate.lock().unwrap().take();
        if let Some(wait) = wait {
            let _ = wait.entered.send(());
            wait.release
                .recv_timeout(Duration::from_secs(4))
                .map_err(|_| ApplicationError::Port(PRIVATE_ERROR.into()))?;
        }
        match *self.mode.lock().unwrap() {
            Mode::Rejected => Err(ApplicationError::InvalidCredentials),
            Mode::MfaRejected => Err(ApplicationError::MfaRejected),
            Mode::Conflict => Err(ApplicationError::ConcurrentModification),
            Mode::InvalidInput => Err(ApplicationError::InvalidInput(PRIVATE_ERROR.into())),
            Mode::Uncertain => Err(ApplicationError::Port(PRIVATE_ERROR.into())),
            _ => Ok(()),
        }
    }
}
fn summary() -> InvitationSummary {
    InvitationSummary {
        id: InvitationId::from_uuid(invitation()),
        revision: REVISION,
        operation_id: operation(),
        email: "staff@example.test".into(),
        role: Role::Litigator,
        status: InvitationStatus::Pending,
        delivery: InvitationDeliveryState::Pending,
        issued_at: OffsetDateTime::UNIX_EPOCH,
        expires_at: OffsetDateTime::from_unix_timestamp(86_400).unwrap(),
        accepted_at: None,
    }
}
fn receipt() -> InvitationReceipt {
    InvitationReceipt {
        id: InvitationId::from_uuid(invitation()),
        revision: REVISION,
        operation_id: operation(),
        user_id: UserId::from_uuid(Uuid::from_u128(51)),
        email: "staff@example.test".into(),
        role: Role::Litigator,
        accepted_at: OffsetDateTime::from_unix_timestamp(60).unwrap(),
    }
}
impl InvitationWorkflow for Workflow {
    fn issue(
        &self,
        token: &str,
        request: InvitationIssueRequest,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.owner(token, Call::Issue(token.into(), request))?;
        Ok(summary())
    }
    fn resend(
        &self,
        token: &str,
        id: InvitationId,
        request: InvitationResendRequest,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.owner(token, Call::Resend(token.into(), id, request))?;
        Ok(summary())
    }
    fn list(
        &self,
        token: &str,
        query: InvitationQuery,
    ) -> Result<InvitationPage, ApplicationError> {
        self.owner(token, Call::List(token.into(), query))?;
        Ok(InvitationPage {
            items: vec![summary()],
            next: Some(InvitationId::from_uuid(invitation())),
        })
    }
    fn get(&self, token: &str, id: InvitationId) -> Result<InvitationSummary, ApplicationError> {
        self.owner(token, Call::Get(token.into(), id))?;
        Ok(summary())
    }
    fn prepare(
        &self,
        token: &[u8],
        id: Uuid,
        password: &str,
    ) -> Result<InvitationEnrollment, ApplicationError> {
        self.recipient("prepare", token, id, password)?;
        Ok(InvitationEnrollment {
            id: InvitationId::from_uuid(invitation()),
            revision: REVISION,
            operation_id: operation(),
            email: "staff@example.test".into(),
            role: Role::Litigator,
            expires_at: OffsetDateTime::from_unix_timestamp(86_400).unwrap(),
            totp_secret_base32: Zeroizing::new(SECRET.into()),
            otpauth_uri: Zeroizing::new(format!("otpauth://totp/staff?secret={SECRET}")),
            recovery_codes: vec![Zeroizing::new("PRIVATE-RECOVERY-CODE".into())],
        })
    }
    fn finish(
        &self,
        token: &[u8],
        id: Uuid,
        password: &str,
        code: &str,
    ) -> Result<InvitationReceipt, ApplicationError> {
        assert_eq!(code, CODE);
        self.recipient("finish", token, id, password)?;
        Ok(receipt())
    }
    fn receipt(
        &self,
        token: &[u8],
        id: Uuid,
        password: &str,
    ) -> Result<Option<InvitationReceipt>, ApplicationError> {
        self.recipient("receipt", token, id, password)?;
        Ok((!matches!(*self.mode.lock().unwrap(), Mode::MissingReceipt)).then(receipt))
    }
}
