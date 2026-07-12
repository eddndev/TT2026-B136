//! Append-only audit trail.
//!
//! The audit trail records actions taken on documents and cases. Each entry is
//! chained to the previous one with a SHA-256 hash so that altering or
//! deleting any past entry breaks verification of the whole trail. The
//! chaining logic and its port are added together with their tests; this
//! module is the home they will live in.
