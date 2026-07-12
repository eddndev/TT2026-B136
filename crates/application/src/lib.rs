//! Use cases that orchestrate the domain.
//!
//! This crate depends only on `domain`. Each use case is a small, testable
//! unit that drives domain values through outbound ports; the ports and their
//! use cases are added together with their tests.

pub mod audit;
pub mod auth;
pub mod error;
pub mod hashing;
pub mod pki;
pub mod vault;

pub use error::ApplicationError;
pub use hashing::HashDocument;
