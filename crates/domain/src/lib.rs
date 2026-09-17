//! Core domain types for the legal case-management prototype.
//!
//! This crate holds entities, value objects, and error types with no
//! dependency on any other workspace crate. Adapters and outbound ports are
//! defined elsewhere; the domain stays free of I/O and framework concerns.

pub mod audit;
pub mod case_administration;
pub mod case_stages;
pub mod cases;
pub mod clock;
pub mod crypto;
pub mod deadline_arithmetic;
pub mod deadline_days;
pub mod deadline_triggers;
pub mod document_metadata;
pub mod error;
pub mod hearing_results;
pub mod hearings;
pub mod identity;
pub mod judicial_calendars;
pub mod participants;
pub mod procedural_facts;
pub mod procedural_time;
pub mod typed_participants;

pub use error::DomainError;
