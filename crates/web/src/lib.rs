//! Inbound HTTP adapter.
//!
//! This crate will expose the use cases over HTTP. It is intentionally empty
//! for now: it compiles as part of the workspace so the driving side of the
//! architecture has a home, and its handlers are added once the use cases they
//! call exist. It may depend only on `application` and `domain`.
