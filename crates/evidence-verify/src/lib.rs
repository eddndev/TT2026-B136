//! Verification of exported Qadra evidence packages.
//!
//! The crate admits a stored ZIP exactly as the server writes it, recognizes
//! its package profile, and checks signatures, certificates, CRLs, time-stamp
//! tokens and canonical statements without I/O, clocks or random sources.
//! Every instant is a Unix time in seconds supplied by the caller or read from
//! the evidence itself. The contract is
//! `docs/adr/0085-in-browser-evidence-validation.md`.

pub mod archive;
pub mod certificate;
mod checks;
pub mod crl;
pub mod der_walk;
pub mod individual_json;
pub mod json;
pub mod manifest;
pub mod pem;
pub mod policy;
pub mod profile;
pub mod receipt;
pub mod report;
pub mod statements;
pub mod timestamp;
mod validate;

pub use policy::Policy;
pub use report::{Finding, Outcome, Report};
pub use validate::validate;
