//! Use cases that orchestrate the domain.
//!
//! This crate depends only on `domain`. Each use case is a small, testable
//! unit that drives domain values through outbound ports; the ports and their
//! use cases are added together with their tests.

pub mod audit;
pub mod auth;
pub mod error;
pub mod evidence;
pub mod hashing;
pub mod pki;
pub mod signing;
pub mod timestamping;
pub mod vault;
pub mod verification;

pub use error::ApplicationError;
pub use hashing::HashDocument;
