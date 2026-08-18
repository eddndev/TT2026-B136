//! Core domain types for the legal case-management prototype.
//!
//! This crate holds entities, value objects, and error types with no
//! dependency on any other workspace crate. Adapters and outbound ports are
//! defined elsewhere; the domain stays free of I/O and framework concerns.

pub mod audit;
pub mod clock;
pub mod crypto;
pub mod error;
pub mod identity;

pub use error::DomainError;
