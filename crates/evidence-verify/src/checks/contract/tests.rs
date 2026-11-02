use super::timing::{self, Stamp, Token, Window};
use crate::report::Outcome;

const WINDOW: Window = Window {
    created_at: 1_000,
    expires_at: 2_000,
};

fn stamp(gen_time: i64) -> Option<Stamp> {
    Some(Stamp {
        gen_time,
        accuracy: 1,
    })
}

fn tokens(t1: Option<Stamp>, td: Option<Stamp>, tf: Option<Stamp>) -> [Token; 3] {
    [
        Token {
            label: "T1",
            stamp: t1,
            bounded: true,
        },
        Token {
            label: "Td",
            stamp: td,
            bounded: false,
        },
        Token {
            label: "Tf",
            stamp: tf,
            bounded: true,
        },
    ]
}

fn failures(window: Option<Window>, tokens: &[Token]) -> Vec<&'static str> {
    timing::check(window, tokens)
        .into_iter()
        .filter(|finding| finding.outcome == Outcome::Failed)
        .map(|finding| finding.code)
        .collect()
}

#[test]
fn ordered_tokens_inside_the_window_pass() {
    let findings = timing::check(
        Some(WINDOW),
        &tokens(stamp(1_000), stamp(1_000), stamp(1_999)),
    );
    assert!(
        findings
            .iter()
            .all(|finding| finding.outcome == Outcome::Checked),
        "{findings:?}"
    );
    assert_eq!(findings.len(), 3);
}

#[test]
fn tokens_out_of_order_fail() {
    for (t1, td, tf) in [(1_100, 1_050, 1_200), (1_100, 1_300, 1_200)] {
        assert_eq!(
            failures(Some(WINDOW), &tokens(stamp(t1), stamp(td), stamp(tf))),
            ["contract.token_order"]
        );
    }
}

#[test]
fn t1_before_the_request_fails() {
    assert_eq!(
        failures(
            Some(WINDOW),
            &tokens(stamp(999), stamp(1_100), stamp(1_200))
        ),
        ["contract.before_issue"]
    );
}

#[test]
fn the_end_of_t1_or_tf_after_the_expiry_fails() {
    // genTime plus accuracy must not pass expires_at; Td is not bounded.
    assert_eq!(
        failures(
            Some(WINDOW),
            &tokens(stamp(1_100), stamp(1_200), stamp(2_000))
        ),
        ["contract.after_expiry"]
    );
    assert_eq!(
        failures(
            Some(WINDOW),
            &tokens(stamp(2_000), stamp(2_000), stamp(2_000))
        ),
        ["contract.after_expiry", "contract.after_expiry"]
    );
}

#[test]
fn an_unverified_token_is_indeterminate_and_skips_its_rules() {
    let findings = timing::check(Some(WINDOW), &tokens(None, stamp(1_100), stamp(1_200)));
    assert!(findings
        .iter()
        .any(|finding| finding.outcome == Outcome::Indeterminate
            && finding.code == "context.token_time_unknown"));
    assert!(failures(Some(WINDOW), &tokens(None, stamp(1_100), stamp(1_200))).is_empty());
    assert!(failures(None, &tokens(stamp(10), stamp(20), stamp(30))).is_empty());
}

#[test]
fn a_registration_after_its_token_fails() {
    assert!(timing::registration("signer-1", 1_001, stamp(1_000)).is_none());
    let finding = timing::registration("signer-1", 1_002, stamp(1_000)).expect("after");
    assert_eq!(
        (finding.outcome, finding.code),
        (Outcome::Failed, "context.registration_after_acceptance")
    );
    assert!(timing::registration("signer-1", 1_002, None).is_none());
}
