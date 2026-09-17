//! Use cases that orchestrate the domain.
//!
//! This crate depends only on `domain`. Each use case is a small, testable
//! unit that drives domain values through outbound ports; the ports and their
//! use cases are added together with their tests.

pub mod audit;
pub mod auth;
pub mod case_stages;
pub mod cases;
pub mod credential_trust;
pub mod deadline_inputs;
pub mod documents;
pub mod error;
pub mod evidence;
pub mod hashing;
pub mod hearing_results;
pub mod hearings;
pub mod identity;
pub mod judicial_calendars;
pub mod participants;
pub mod pki;
pub mod procedural_facts;
pub mod signing;
pub mod timestamping;
pub mod typed_participants;
pub mod vault;
pub mod verification;

pub use error::ApplicationError;
pub use hashing::HashDocument;
