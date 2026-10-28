//! What the caller supplies to a validation: the pinned roots, the trust the
//! server currently serves, the admitted time-stamp policies and the instant
//! of the current status.
//!
//! A root inside a package is never trusted because it is present: it is
//! admitted only when its SHA-256 fingerprint is pinned for the purpose it
//! serves. The served trust is compared with the pinned set and supplies the
//! current CRLs. Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

/// The time-stamp policy of the internal authority, admitted by default.
pub const DEFAULT_ADMITTED_POLICY: &str = "1.3.6.1.4.1.13762.3";

/// What a root anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The issuer of the server seal signer.
    ServerSeal,
    /// The credential trust root of individual signing holders.
    IndividualSigning,
    /// The issuer of the time-stamping certificate.
    TimeStamping,
}

impl Purpose {
    /// The purpose name used by the pinned roots file and the trust route.
    pub fn name(self) -> &'static str {
        match self {
            Self::ServerSeal => "server-seal",
            Self::IndividualSigning => "individual-signing",
            Self::TimeStamping => "time-stamping",
        }
    }

    pub const ALL: [Purpose; 3] = [
        Purpose::ServerSeal,
        Purpose::IndividualSigning,
        Purpose::TimeStamping,
    ];
}

/// The set of purposes one root serves; a root may serve several.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Purposes {
    pub server_seal: bool,
    pub individual_signing: bool,
    pub time_stamping: bool,
}

impl Purposes {
    /// Every purpose.
    pub const ALL: Purposes = Purposes {
        server_seal: true,
        individual_signing: true,
        time_stamping: true,
    };

    /// Whether `purpose` is in the set.
    pub fn allows(self, purpose: Purpose) -> bool {
        match purpose {
            Purpose::ServerSeal => self.server_seal,
            Purpose::IndividualSigning => self.individual_signing,
            Purpose::TimeStamping => self.time_stamping,
        }
    }
}

/// One root built into the release, identified by its SHA-256 fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedRoot {
    /// SHA-256 of the root certificate DER.
    pub sha256: [u8; 32],
    pub purposes: Purposes,
    /// A human label shown in the report.
    pub label: String,
}

/// One root the trust route returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedRoot {
    /// The root certificate DER.
    pub der: Vec<u8>,
    pub purposes: Purposes,
}

/// What the server currently admits, as returned by its trust route.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServedTrust {
    pub roots: Vec<ServedRoot>,
    /// Current CRLs in DER, used only for the current status.
    pub crls: Vec<Vec<u8>>,
}

/// The input of one validation besides the package bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub pinned: Vec<PinnedRoot>,
    /// The served trust, or `None` when the route was unavailable; then only
    /// the current status changes.
    pub served: Option<ServedTrust>,
    /// Dotted time-stamp policies admitted for tokens.
    pub admitted_policies: Vec<String>,
    /// Unix seconds of the current status, read from the client clock by
    /// the caller; it never decides the historical result.
    pub current_instant: i64,
}

impl Policy {
    /// A policy with the given pinned roots, no served trust and the
    /// default admitted time-stamp policy.
    pub fn new(pinned: Vec<PinnedRoot>, current_instant: i64) -> Self {
        Self {
            pinned,
            served: None,
            admitted_policies: vec![DEFAULT_ADMITTED_POLICY.to_owned()],
            current_instant,
        }
    }

    /// The pinned root with fingerprint `sha256` for `purpose`, if any.
    pub fn pinned_for(&self, sha256: &[u8; 32], purpose: Purpose) -> Option<&PinnedRoot> {
        self.pinned
            .iter()
            .find(|root| &root.sha256 == sha256 && root.purposes.allows(purpose))
    }
}
