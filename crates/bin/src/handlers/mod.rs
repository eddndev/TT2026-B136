//! Subcommand handlers.
//!
//! Each handler wires concrete adapters into a use case and formats the
//! result for the terminal; no domain or use-case logic lives here. Handlers
//! for further subcommands are added as sibling modules.

mod crypto;

pub use crypto::hash_file;
