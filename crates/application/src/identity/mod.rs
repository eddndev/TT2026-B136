//! Multi-user enrollment, authentication, sessions, and authorization.

pub mod certificate_login;
mod model;
mod observation;
pub mod owner_certificates;
pub mod password_reset;
mod port;
mod service;
mod session;
mod validation;
mod workflow;

pub use model::{
    EnrollmentResult, LoginChallenge, LoginChallengeIdentity, Principal, SessionIdentity,
    SessionResult, UserRecord,
};
pub use observation::{MfaAttempt, MfaReason};
pub use port::{IdentityWorkflow, SecretProtector, SessionStore, UserRepository};
pub use service::{IdentityPorts, IdentityService};
pub use session::{SessionGrant, SessionPolicy, SessionState, SessionStatus};
pub(crate) use validation::normalize_email;
