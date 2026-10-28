//! Result vocabulary shared by every check, and the report of one
//! validation.

mod summary;

pub use summary::{
    hex, Component, ComponentKind, Instant, InstantSource, Profile, Report, TrustNote,
};

/// The result of one check or component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The check ran over the available evidence and passed.
    Checked,
    /// The check ran and the evidence contradicts the claim.
    Failed,
    /// The evidence needed to run the check is missing or out of its window.
    Indeterminate,
    /// The input uses a form or authority this verifier does not admit.
    NotSupported,
    /// The check does not belong to the recognized profile.
    NotApplicable,
}

/// One outcome with a stable machine-readable code and a human detail.
///
/// `code` is a short ASCII identifier such as `token.imprint_mismatch`; the
/// interface translates it. `detail` adds context such as an entry name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub outcome: Outcome,
    pub code: &'static str,
    pub detail: String,
}

impl Finding {
    pub fn new(outcome: Outcome, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            outcome,
            code,
            detail: detail.into(),
        }
    }

    pub fn checked(code: &'static str) -> Self {
        Self::new(Outcome::Checked, code, "")
    }

    pub fn failed(code: &'static str, detail: impl Into<String>) -> Self {
        Self::new(Outcome::Failed, code, detail)
    }

    pub fn indeterminate(code: &'static str, detail: impl Into<String>) -> Self {
        Self::new(Outcome::Indeterminate, code, detail)
    }

    pub fn not_supported(code: &'static str, detail: impl Into<String>) -> Self {
        Self::new(Outcome::NotSupported, code, detail)
    }
}

/// Combine mandatory outcomes with the precedence of the contract: failed,
/// then not supported, then indeterminate, otherwise checked. Not applicable
/// outcomes are ignored; an empty set is not applicable.
pub fn combine(outcomes: impl IntoIterator<Item = Outcome>) -> Outcome {
    let mut seen = Outcome::NotApplicable;
    for outcome in outcomes {
        seen = match (seen, outcome) {
            (_, Outcome::Failed) | (Outcome::Failed, _) => Outcome::Failed,
            (_, Outcome::NotSupported) | (Outcome::NotSupported, _) => Outcome::NotSupported,
            (_, Outcome::Indeterminate) | (Outcome::Indeterminate, _) => Outcome::Indeterminate,
            (_, Outcome::Checked) | (Outcome::Checked, _) => Outcome::Checked,
            _ => Outcome::NotApplicable,
        };
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_follows_the_contract_precedence() {
        use Outcome::*;
        assert_eq!(combine([]), NotApplicable);
        assert_eq!(combine([NotApplicable, Checked]), Checked);
        assert_eq!(combine([Checked, Indeterminate]), Indeterminate);
        assert_eq!(combine([Indeterminate, NotSupported]), NotSupported);
        assert_eq!(combine([NotSupported, Failed, Checked]), Failed);
        assert_eq!(combine([Failed, NotApplicable]), Failed);
    }
}
