use application::{
    identity::{
        certificate_login::{CertificateLoginChallenge, OwnerLoginWorkflow},
        LoginChallenge,
    },
    ApplicationError,
};
use axum::{body::Body, http::Request, Router};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
    owner_certificate_login::{LoginAccount, LoginNonce, LoginStatement},
    owner_certificates::BindingMaterial,
};
use std::{
    num::NonZeroUsize,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;
use web::HttpLimits;

pub use crate::password_reset_http_support::Reply;
pub const START: &str = "/api/v1/auth/certificate-login/start";
pub const PROOF: &str = "/api/v1/auth/certificate-login/proof";
pub const OWNER: &str = "a20cfde1-17dd-4bfb-88ce-f306cb85fa80";
pub const BINDING: &str = "5bb5232b-4cca-4c45-b681-61d23d1a7cf0";

pub fn token() -> String {
    URL_SAFE_NO_PAD.encode([0xfb; 32])
}
pub fn mfa_token() -> String {
    URL_SAFE_NO_PAD.encode([0x12; 32])
}
pub fn signature() -> Vec<u8> {
    (0..384).map(|index| (index % 256) as u8).collect()
}
pub fn start_body() -> String {
    serde_json::json!({"owner_id": OWNER, "binding_id": BINDING}).to_string()
}
pub fn proof_body() -> String {
    serde_json::json!({"challenge_token": token(), "signature_base64": STANDARD.encode(signature())}).to_string()
}

// Synthetic public values exercise HTTP projection; no credential is verified here.
pub fn statement() -> LoginStatement {
    LoginStatement::new(
        LoginAccount::new(
            UserId::from_uuid(Uuid::parse_str(OWNER).unwrap()),
            Role::Owner,
            true,
            9_007_199_254_740_993,
        )
        .unwrap(),
        BindingMaterial::new(
            Uuid::from_u128(19),
            Uuid::parse_str(BINDING).unwrap(),
            Sha256Digest::from_array([0x31; 32]),
            Sha256Digest::from_array([0x57; 32]),
            7,
        )
        .unwrap(),
        LoginNonce::from_bytes(&[0xe3; 32]).unwrap(),
        1_900_000_000,
        1_900_000_300,
    )
    .unwrap()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Start(UserId, Uuid),
    Proof(String, Vec<u8>),
}
#[derive(Clone, Copy, Default)]
pub enum Outcome {
    #[default]
    Challenge,
    Invalid,
    Locked,
    Port,
}

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<Call>>,
    pub outcome: Mutex<Outcome>,
    wait: Mutex<Option<Wait>>,
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
            .expect("certificate proof did not enter its workflow")
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
    pub fn block_proof(&self) -> Gate {
        let (entered, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = mpsc::channel();
        *self.wait.lock().unwrap() = Some(Wait {
            entered,
            release: wait,
        });
        Gate {
            entered: ready,
            release: Some(release),
        }
    }
    fn result(&self) -> Result<(), ApplicationError> {
        match *self.outcome.lock().unwrap() {
            Outcome::Challenge => Ok(()),
            Outcome::Invalid => Err(ApplicationError::InvalidCredentials),
            Outcome::Locked => Err(ApplicationError::AccountLocked),
            Outcome::Port => Err(ApplicationError::Port(format!(
                "private-owner-login-port {}",
                token()
            ))),
        }
    }
}
impl OwnerLoginWorkflow for Workflow {
    fn start_certificate_login(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<CertificateLoginChallenge, ApplicationError> {
        self.calls.lock().unwrap().push(Call::Start(owner, binding));
        self.result()?;
        Ok(CertificateLoginChallenge {
            challenge_token: token(),
            statement: statement(),
            expires_in_seconds: 299,
        })
    }
    fn prove_certificate_login(
        &self,
        token: &str,
        signature: &[u8],
    ) -> Result<LoginChallenge, ApplicationError> {
        self.calls
            .lock()
            .unwrap()
            .push(Call::Proof(token.to_owned(), signature.to_vec()));
        let wait = self.wait.lock().unwrap().take();
        if let Some(wait) = wait {
            let _ = wait.entered.send(());
            wait.release
                .recv_timeout(Duration::from_secs(4))
                .expect("certificate workflow gate was not released");
        }
        self.result()?;
        Ok(LoginChallenge {
            challenge_token: mfa_token(),
            expires_in_seconds: 300,
        })
    }
}

pub fn limits(requests: usize, blocking: usize) -> HttpLimits {
    HttpLimits {
        max_requests: NonZeroUsize::new(requests).unwrap(),
        max_blocking_operations: NonZeroUsize::new(blocking).unwrap(),
    }
}
pub fn standalone() -> (Router, Arc<Workflow>) {
    let workflow = Arc::new(Workflow::default());
    (
        web::owner_login_router(workflow.clone(), limits(2, 1)),
        workflow,
    )
}
pub async fn send(router: &Router, request: Request<Body>) -> Reply {
    use tower::ServiceExt;
    let response = tokio::time::timeout(Duration::from_secs(2), router.clone().oneshot(request))
        .await
        .expect("certificate HTTP did not finish within the test bound")
        .unwrap();
    Reply {
        status: response.status(),
        headers: response.headers().clone(),
        body: axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap()
            .to_vec(),
    }
}
pub async fn post(router: &Router, path: &str, body: String) -> Reply {
    send(
        router,
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap(),
    )
    .await
}
pub fn public(reply: &Reply) {
    assert_eq!(reply.headers["cache-control"], "no-store");
    for name in ["set-cookie", "authorization", "location"] {
        assert!(!reply.headers.contains_key(name));
    }
}
