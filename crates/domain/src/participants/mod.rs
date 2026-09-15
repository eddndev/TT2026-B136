//! Case-local directory values, independent of user accounts and access grants.

mod identity;
mod values;

pub use identity::{ParticipantId, ParticipantRevision};
pub use values::{DirectoryStatus, ParticipantValues};
