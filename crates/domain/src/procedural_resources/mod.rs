//! Declared remedies and acts retain evidence without inferring procedural effects.
mod acts;
mod canonical;
mod catalog;
mod encoding;
mod identity;
mod roots;
mod values;

pub use acts::{ResourceActValues, ResourceActValuesInput, MAX_RESOURCE_ACT_EVIDENCE};
pub use catalog::{ResourceActKind, ResourceKind, ResourceMode, ResourceStatus};
pub use identity::{
    ResourceActId, ResourceActRevision, ResourceId, ResourceOperationId, ResourceRevision,
};
pub use roots::{ResourceActRoot, ResourceRoot};
pub use values::{ResourceAppellant, ResourceValues, ResourceValuesInput, MAX_RESOURCE_APPELLANTS};
