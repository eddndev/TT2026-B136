//! The order of the tokens and the validity window of the request.
//!
//! The tokens must satisfy `T1 <= Td <= Tf` by `genTime`, T1 must not be
//! before the request's `created_at`, and the end of the T1 and Tf intervals
//! (`genTime` plus accuracy) must not pass `expires_at`: using `genTime`
//! minus accuracy would hand the firm extra margin. A binding registered
//! after the end of its party's token interval fails. Rationale: the
//! sections "Time-stamps" and "Evidence packages" of
//! `docs/adr/0086-bilateral-contract-signing.md`.

use crate::report::{Finding, Outcome};
use crate::timestamp::TokenFacts;

/// The verified `genTime` and accuracy of one token, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Stamp {
    pub gen_time: i64,
    pub accuracy: i64,
}

impl Stamp {
    pub fn of(facts: &TokenFacts) -> Self {
        Self {
            gen_time: facts.gen_time,
            accuracy: facts.accuracy_seconds,
        }
    }

    fn end(self) -> i64 {
        self.gen_time.saturating_add(self.accuracy)
    }
}

/// The validity the request states, in Unix seconds.
#[derive(Debug, Clone, Copy)]
pub(super) struct Window {
    pub created_at: i64,
    pub expires_at: i64,
}

/// One token in signing order.
pub(super) struct Token {
    pub label: &'static str,
    /// `None` when the token did not yield verified facts.
    pub stamp: Option<Stamp>,
    /// Whether the end of its interval must not pass `expires_at`.
    pub bounded: bool,
}

/// The time of each token, their order and the window, when the request
/// decoded; the first token is T1.
pub(super) fn check(window: Option<Window>, tokens: &[Token]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for token in tokens {
        findings.push(match token.stamp {
            Some(stamp) => Finding::new(
                Outcome::Checked,
                "context.token_time",
                format!(
                    "{} genTime {} accuracy {} s",
                    token.label, stamp.gen_time, stamp.accuracy
                ),
            ),
            None => Finding::indeterminate(
                "context.token_time_unknown",
                format!("{} did not yield a verified genTime", token.label),
            ),
        });
    }
    for pair in tokens.windows(2) {
        if let (Some(earlier), Some(later)) = (pair[0].stamp, pair[1].stamp) {
            if earlier.gen_time > later.gen_time {
                findings.push(Finding::failed(
                    "contract.token_order",
                    format!(
                        "{} genTime {} is after {} genTime {}",
                        pair[0].label, earlier.gen_time, pair[1].label, later.gen_time
                    ),
                ));
            }
        }
    }
    let Some(window) = window else {
        return findings;
    };
    if let Some(first) = tokens.first() {
        if let Some(stamp) = first
            .stamp
            .filter(|stamp| window.created_at > stamp.gen_time)
        {
            findings.push(Finding::failed(
                "contract.before_issue",
                format!(
                    "{} genTime {} is before the request created_at {}",
                    first.label, stamp.gen_time, window.created_at
                ),
            ));
        }
    }
    for token in tokens.iter().filter(|token| token.bounded) {
        if let Some(stamp) = token.stamp.filter(|stamp| stamp.end() > window.expires_at) {
            findings.push(Finding::failed(
                "contract.after_expiry",
                format!(
                    "{} genTime {} plus accuracy {} s passes expires_at {}",
                    token.label, stamp.gen_time, stamp.accuracy, window.expires_at
                ),
            ));
        }
    }
    findings
}

/// A binding registered after the end of its party's token interval fails;
/// without a verified token nothing is concluded here.
pub(super) fn registration(
    label: &str,
    registered_at: i64,
    signing: Option<Stamp>,
) -> Option<Finding> {
    let stamp = signing?;
    (registered_at > stamp.end()).then(|| {
        Finding::failed(
            "context.registration_after_acceptance",
            format!(
                "{label} binding_registered_at {registered_at} is after its token genTime {} plus accuracy {} s",
                stamp.gen_time, stamp.accuracy
            ),
        )
    })
}
