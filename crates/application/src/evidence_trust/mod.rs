//! Read-only trust material the server currently admits for exported evidence.
//!
//! The in-browser validator contrasts it with the roots pinned in the release;
//! it never makes a root trusted by itself. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.
mod model;
mod port;
mod service;
pub use model::*;
pub use port::*;
pub use service::*;
