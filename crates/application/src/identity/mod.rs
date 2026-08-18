//! Multi-user enrollment, authentication, sessions, and authorization.

mod model;
mod port;
mod service;
mod workflow;

pub use model::{EnrollmentResult, LoginChallenge, Principal, SessionResult, UserRecord};
pub use port::{IdentityWorkflow, SecretProtector, SessionStore, UserRepository};
pub use service::{IdentityPorts, IdentityService};
