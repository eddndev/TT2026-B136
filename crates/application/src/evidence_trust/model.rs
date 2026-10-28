use std::sync::Arc;

use crate::credential_trust::CredentialTrustSnapshot;

/// Public seal and time-stamping anchors decoded once from the configured evidence material.
///
/// Fingerprints and digests are lowercase hexadecimal SHA-256 of the DER bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedEvidenceMaterial {
    pub seal_root_der: Vec<u8>,
    pub seal_root_fingerprint: String,
    pub seal_crl_der: Vec<u8>,
    pub seal_crl_digest: String,
    /// Every certificate of the time-stamping chain as `(der, fingerprint)`, in file order.
    pub time_stamping: Vec<(Vec<u8>, String)>,
}

/// What the server admits now: the static material and the current credential trust, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceTrust {
    pub material: Arc<ServedEvidenceMaterial>,
    pub individual_signing: Option<CredentialTrustSnapshot>,
}
