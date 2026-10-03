use application::identity::owner_certificates::OwnerBindingCommit;
use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    identity::Role,
    owner_certificates::{BindingStatement, OwnerAccount},
};
use serde_json::json;

use crate::{owner_certificate_http_support::*, submission_support, support::*};

#[tokio::test]
async fn preparation_preserves_canonical_bytes_and_decimal_account_counters() {
    let harness = Harness::new();
    let observed = harness.state.clone();
    let revision = 9_007_199_254_740_993;
    let generation = 9_007_199_254_740_991;
    observed.lock().unwrap().context.account.revision = revision;
    observed.lock().unwrap().context.account.auth_generation = generation;
    let expected = BindingStatement::new(
        OwnerAccount::new(principal().id, Role::Owner, true, revision, generation).unwrap(),
        principal().id,
        *statement().material(),
    )
    .unwrap();
    let response = request(&router(harness), "POST", "/prepare", Some(prepare_body())).await;
    response.public();
    assert_eq!(response.status, StatusCode::OK);
    let value = response.json();
    assert_eq!(value["binding_id"], binding().to_string());
    assert_eq!(value["owner_id"], principal().id.to_string());
    assert_eq!(value["policy"], "internal_partner_binding_v1");
    assert_eq!(
        decoded(&value["statement_base64"]),
        expected.canonical_bytes()
    );
    assert_eq!(decoded(&value["statement_base64"]).len(), 150);
    assert!(value.get("statement_digest").is_none());
    assert_eq!(value["account_revision"], revision.to_string());
    assert_eq!(value["auth_generation"], generation.to_string());
    assert_eq!(
        decoded(&value["certificate"]["der_base64"]),
        certificate().der
    );
    assert_eq!(
        value["certificate"]["fingerprint"],
        certificate().fingerprint.to_hex()
    );
    assert_eq!(
        value["certificate"]["summary"]["serial_hex"],
        certificate().summary.serial_hex
    );
    assert_eq!(value["deployment_id"], trust().deployment_id.to_string());
    assert_eq!(value["trust_revision"], trust().revision.get());
    assert_eq!(
        value["root_fingerprint"],
        trust().inspection.root_fingerprint.to_hex()
    );
    submission_support::no_crypto_or_commit(&observed);
    assert_eq!(submission_support::count(&observed, "inspect"), 1);
}

#[tokio::test]
async fn registration_submits_exact_bytes_once_and_replays_original_terminal_evidence() {
    let mut harness = submission_support::harness();
    let observed = harness.state.clone();
    harness.verify_with(|_, _| {});
    let calls = observed.clone();
    harness
        .store
        .expect_commit_registration()
        .times(1)
        .returning(move |command| {
            assert_eq!(command.registration().statement(), &statement());
            assert_eq!(command.registration().certificate(), &certificate());
            assert_eq!(command.check().signature, signature());
            let mut state = calls.lock().unwrap();
            state.calls.push("commit_registration");
            state.found = Some(receipt());
            Ok(OwnerBindingCommit::Applied(receipt()))
        });
    let app = router(harness);
    let first = request(&app, "POST", "/register", Some(registration_body())).await;
    first.public();
    assert_eq!(first.status, StatusCode::OK);
    assert_evidence(&first.json(), &receipt());
    let before = observed.lock().unwrap().calls.clone();
    {
        let mut state = observed.lock().unwrap();
        state.found = Some(retired());
        state.now = 4000;
        state.context.trust = None;
    }
    let replay = request(&app, "POST", "/register", Some(registration_body())).await;
    replay.public();
    assert_eq!(replay.status, StatusCode::OK);
    assert_evidence(&replay.json(), &retired());
    let mut altered = registration_body();
    let mut bytes = statement().canonical_bytes();
    bytes[149] ^= 1;
    altered["statement_base64"] = json!(STANDARD.encode(bytes));
    request(&app, "POST", "/register", Some(altered))
        .await
        .error(StatusCode::CONFLICT, "owner_certificate_conflict");
    for call in [
        "inspect",
        "load_registration",
        "verify",
        "commit_registration",
    ] {
        assert_eq!(
            submission_support::count(&observed, call),
            before.iter().filter(|v| **v == call).count()
        );
    }
}

#[tokio::test]
async fn exact_receipt_and_terminal_withdrawal_preserve_history_after_trust_expiry() {
    let mut harness = Harness::new();
    let observed = harness.state.clone();
    let mut original_receipt = receipt();
    original_receipt.registered_at = at(1000).replace_nanosecond(123_456_789).unwrap();
    let mut terminal_receipt = retired();
    terminal_receipt.registered_at = original_receipt.registered_at;
    terminal_receipt.withdrawn_at = Some(at(3000).replace_nanosecond(987_654_321).unwrap());
    {
        let mut state = observed.lock().unwrap();
        state.found = Some(original_receipt.clone());
        state.context.account.revision = 10;
        state.context.trust = None;
        state.now = 3000;
    }
    harness.withdrawal(original_receipt.clone());
    let calls = observed.clone();
    let expected_original = original_receipt.clone();
    let committed = terminal_receipt.clone();
    harness
        .store
        .expect_commit_withdrawal()
        .times(1)
        .returning(move |prepared| {
            assert_eq!(prepared.original(), &expected_original);
            assert_eq!(prepared.record(), &committed.record);
            assert_eq!(prepared.not_before(), at(3000));
            let mut state = calls.lock().unwrap();
            state.calls.push("commit_withdrawal");
            state.found = Some(committed.clone());
            Ok(OwnerBindingCommit::Applied(committed.clone()))
        });
    let app = router(harness);
    let original = request(&app, "GET", "", None).await;
    original.public();
    assert_eq!(original.status, StatusCode::OK);
    assert_evidence(&original.json(), &original_receipt);
    let terminal = request(
        &app,
        "POST",
        "/withdraw",
        Some(json!({"expected_revision":1})),
    )
    .await;
    terminal.public();
    assert_eq!(terminal.status, StatusCode::OK);
    assert_evidence(&terminal.json(), &terminal_receipt);
    let read = request(&app, "GET", "", None).await;
    read.public();
    assert_eq!(read.status, StatusCode::OK);
    assert_eq!(read.json(), terminal.json());
    submission_support::no_crypto_or_commit(&observed);
    assert_eq!(submission_support::count(&observed, "commit_withdrawal"), 1);
}

#[tokio::test]
async fn another_path_uuid_never_reinterprets_a_signed_statement() {
    let mut harness = submission_support::harness();
    let observed = harness.state.clone();
    let other = uuid::Uuid::from_u128(45);
    submission_support::replace_store(&mut harness, other, |_| {}, |state| state.context.clone());
    let response = raw(
        &router(harness),
        "POST",
        &format!("/api/v1/auth/certificate-bindings/{other}/register"),
        registration_body().to_string().into_bytes(),
        &["application/json"],
        &["Bearer session"],
    )
    .await;
    response.error(StatusCode::CONFLICT, "owner_certificate_conflict");
    submission_support::no_crypto_or_commit(&observed);
}
