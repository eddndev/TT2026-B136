//! Multi-user enrollment, authentication, sessions, and authorization.

mod model;
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
pub use port::{IdentityWorkflow, SecretProtector, SessionStore, UserRepository};
pub use service::{IdentityPorts, IdentityService};
pub use session::{SessionGrant, SessionPolicy, SessionState, SessionStatus};
pub(crate) use validation::normalize_email;
