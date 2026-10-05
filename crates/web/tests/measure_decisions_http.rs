#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/precautionary_context_support/mod.rs"]
mod context_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_correction_capture_support/mod.rs"]
mod correction_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_decision_effect_support/mod.rs"]
mod effect_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_source_support/mod.rs"]
mod measure_source_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/precautionary_participant_support/mod.rs"]
mod participant_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_record_decision_support/mod.rs"]
mod record_decision_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/measure_record_history_support/mod.rs"]
mod record_support;
use measure_decision_fixtures as decision_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/decision_anchor_support/mod.rs"]
mod decision_anchor_support;
#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/precautionary_receipt_support/mod.rs"]
mod precautionary_receipt_support;

mod measure_decisions_http_support;
use measure_decisions_http_support::*;

#[tokio::test]
async fn corrected_record_prepares_and_submits_a_genuine_g2_receipt() {
    let stored = g2();
    let expected = review(&stored);
    let returned = expected.clone();
    let mut write = MockWrite::new();
    write
        .expect_prepare()
        .times(1)
        .return_once(move |token, case, value| {
            assert_eq!((token, case), ("staff-token", case_id()));
            assert_eq!(&value, returned.command());
            Ok(returned)
        });
    let (status, value) = request(
        write,
        MockRead::new(),
        "POST",
        &format!("{}/prepare", base()),
        Some(command(expected.command())),
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["family"], "g2");
    assert_eq!(value["review"]["command"], command(expected.command()));
    assert_eq!(value["review"]["results"][0]["revision"], 3);
    assert_eq!(
        value["review"]["results"][0]["values"]["conditions"],
        "Corrected recorded conditions"
    );
    assert_eq!(
        value["review"]["submission_digest"],
        expected.submission_digest().to_hex()
    );
    assert_eq!(
        value["review"]["review_digest"],
        expected.review_digest().to_hex()
    );

    let returned = stored.clone();
    let mut write = MockWrite::new();
    write
        .expect_submit()
        .times(1)
        .return_once(move |token, case, value, confirmation| {
            assert_eq!((token, case), ("staff-token", case_id()));
            assert_eq!(&value, returned.command());
            assert_eq!(confirmation.submission_digest, returned.submission_digest());
            assert_eq!(confirmation.review_digest, returned.review_digest());
            Ok(returned)
        });
    let (status, value) = request(
        write,
        MockRead::new(),
        "POST",
        &format!("{}/submit", base()),
        Some(submission(&stored)),
    )
    .await;
    assert_eq!(status, 201, "{value}");
    assert_eq!(value["family"], "g2");
    assert_eq!(value["group"]["family"], "g2");
    assert_eq!(value["group"]["measures"][0]["family"], "m2");
    assert_eq!(
        value["origin"]["group_digest"],
        stored.origin().group_digest.to_hex()
    );
    assert_eq!(
        value["record_history"]["records"]["judicial"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        value["record_history"]["records"]["administrative"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
