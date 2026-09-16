//! Declared resolutions and notification practices do not establish legal effects.

mod canonical;
mod catalog;
mod encoding;
mod evidence;
mod identity;
mod notification;
mod people;
mod provenance;
mod resolution;
mod roots;
mod text;

pub use catalog::{
    FactDeclaration, NotificationCharacter, NotificationContext, NotificationMedium,
    NotificationOutcome, ResolutionClass,
};
pub use evidence::{FactEvidence, FactSupportRef};
pub use identity::{FactOperationId, FactRevision, NotificationId, ResolutionId};
pub use notification::{FactStatedEffect, NotificationValues, NotificationValuesInput};
pub use people::{FactParticipantRef, FactPerson, FactRepresentation};
pub use provenance::{FactHearingRef, FactProvenance};
pub use resolution::{FactResolutionRef, ResolutionValues, ResolutionValuesInput};
pub use roots::{NotificationRoot, ResolutionRoot};
pub use text::{FactLabel, FactText};

/// Smallest PFRES1 declaration, with one-byte text fields and unknown time.
pub const MIN_RESOLUTION_CANONICAL_BYTES: usize = 27;
/// Largest PFRES1 declaration, including four-byte Unicode scalar values.
pub const MAX_RESOLUTION_CANONICAL_BYTES: usize = 17700;
/// Smallest PFNOT1 declaration, with explicit unknown persons and no optional times.
pub const MIN_NOTIFICATION_CANONICAL_BYTES: usize = 67;
/// Largest PFNOT1 declaration, including all optional fields and exact supports.
pub const MAX_NOTIFICATION_CANONICAL_BYTES: usize = 58671;
