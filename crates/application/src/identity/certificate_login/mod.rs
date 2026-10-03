//! Owner certificate first factor and explicit authentication provenance.
//!
//! A certificate proof only grants a new MFA challenge. Current authority is
//! checked again at MFA and session admission. See docs/adr/0068-owner-certificate-first-factor.md.

mod model;
mod port;
pub(crate) mod validation;

pub use model::{
    CertificateLoginChallenge, CertificateLoginContext, CertificateMfaChallenge,
    CertificateSessionProvenance, MfaChallenge, SessionAuthentication, StoredCertificateLogin,
};
pub use port::{
    CertificateLoginPorts, OwnerLoginAuthority, OwnerLoginRuntime, OwnerLoginVerifier,
    OwnerLoginWorkflow,
};
