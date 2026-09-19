use crate::cases::CurrentCaseAdministration;
use domain::crypto::{DocumentHasher, Sha256Digest};

/// Commit the same complete administration evidence used by tracked captures.
/// The caller must authenticate the historical administration before using it.
pub fn administration_evidence_digest(
    hasher: &dyn DocumentHasher,
    administration: &CurrentCaseAdministration,
) -> Sha256Digest {
    let mut bytes = Vec::new();
    crate::deadlines::evidence::administration(&mut bytes, hasher, administration);
    hasher.hash_bytes(&bytes)
}
