//! Multi-user enrollment, authentication, sessions, and authorization.

mod model;
mod port;
mod service;
mod validation;
mod workflow;

pub use model::{
    EnrollmentResult, LoginChallenge, LoginChallengeIdentity, Principal, SessionIdentity,
    SessionResult, UserRecord,
};
pub use port::{IdentityWorkflow, SecretProtector, SessionStore, UserRepository};
pub use service::{IdentityPorts, IdentityService};
pub(crate) use validation::normalize_email;
