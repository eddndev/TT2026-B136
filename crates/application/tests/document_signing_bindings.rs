#[allow(dead_code)]
mod case_support;
mod document_signing_support;
use document_signing_support::*;

use application::{document_signing::SigningWorkflowError as Error, ApplicationError};
use domain::{
    crypto::{CredentialFailure, Signature},
    document_signing::SignerAccount,
    identity::{Role, UserId},
};
use uuid::Uuid;

#[test]
fn preparation_captures_the_exact_principal_account_certificate_and_trust() {
    let harness = Harness::new();
    let state = harness.state.clone();
    let prepared = harness
        .service()
        .prepare("session", id(), &certificate().der)
        .unwrap();
    assert_eq!(prepared.context(), &context());
    assert_eq!(prepared.statement(), &statement());
    assert_eq!(prepared.certificate(), &certificate());
    assert_eq!(prepared.trust(), &trust());
    assert!(!state.lock().unwrap().calls.contains(&"verify"));
}

#[test]
fn external_proof_is_checked_before_an_opaque_commit_and_returns_the_captured_receipt() {
    let mut harness = Harness::new();
    harness.verify_with(|_, _| {});
    harness
        .store
        .expect_commit_binding()
        .times(1)
        .returning(|verified| {
            assert_eq!(verified.prepared().context(), &context());
            assert_eq!(verified.prepared().statement(), &statement());
            assert_eq!(verified.check(), &receipt().check);
            assert_eq!(verified.not_before(), at(1000));
            Ok(receipt())
        });
    assert_eq!(
        harness
            .service()
            .register("session", &statement(), &certificate().der, &signature())
            .unwrap(),
        receipt()
    );
}

#[test]
fn exact_replay_returns_the_terminal_receipt_without_current_trust_or_new_verification() {
    let harness = Harness::new();
    let mut retired = receipt();
    retired.withdrawn_at = Some(at(3000));
    {
        let mut state = harness.state.lock().unwrap();
        state.now = 5000;
        state.context.trust = None;
        state.found = Some(retired.clone());
        state.context.account =
            SignerAccount::new(principal().id, Role::Litigator, true, 12, 7).unwrap();
    }
    let state = harness.state.clone();
    assert_eq!(
        harness
            .service()
            .register("session", &statement(), &certificate().der, &signature())
            .unwrap(),
        retired
    );
    assert!(!state.lock().unwrap().calls.contains(&"context"));
}

#[test]
fn reusing_a_binding_id_with_another_signature_is_a_conflict() {
    let harness = Harness::new();
    harness.state.lock().unwrap().found = Some(receipt());
    let other = Signature::from_bytes(vec![6; 384]).unwrap();
    assert_eq!(
        kind(failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &other
        ))),
        Error::Conflict
    );
}

#[test]
fn new_registration_rejects_changed_account_or_trust_without_cryptographic_work() {
    let harness = Harness::new();
    harness.state.lock().unwrap().context.account =
        SignerAccount::new(principal().id, Role::Litigator, true, 10, 4).unwrap();
    assert_eq!(
        kind(failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &signature()
        ))),
        Error::AccountChanged
    );
    let harness = Harness::new();
    harness
        .state
        .lock()
        .unwrap()
        .context
        .trust
        .as_mut()
        .unwrap()
        .revision = application::credential_trust::CredentialTrustRevision::new(8).unwrap();
    assert_eq!(
        kind(failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &signature()
        ))),
        Error::TrustChanged
    );
}

#[test]
fn unavailable_trust_and_expired_or_revoked_proof_never_commit() {
    let harness = Harness::new();
    harness.state.lock().unwrap().context.trust = None;
    assert_eq!(
        kind(failure(harness.service().prepare(
            "session",
            id(),
            &certificate().der
        ))),
        Error::TrustUnavailable
    );
    for rejection in [
        CredentialFailure::Expired,
        CredentialFailure::Revoked,
        CredentialFailure::CrlExpired,
    ] {
        let mut harness = Harness::new();
        harness
            .verifier
            .expect_verify_binding()
            .times(1)
            .returning(move |_, _, _, _, _| Err(Error::Credential(rejection)));
        assert_eq!(
            kind(failure(harness.service().register(
                "session",
                &statement(),
                &certificate().der,
                &signature()
            ))),
            Error::Credential(rejection)
        );
    }
}

#[test]
fn principal_changes_after_verification_or_receipt_lookup_cannot_be_confirmed() {
    let mut harness = Harness::new();
    harness.verify_with(|_, state| {
        state.lock().unwrap().principal.as_mut().unwrap().email = "other@example.test".into()
    });
    assert!(matches!(
        failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &signature()
        )),
        ApplicationError::InvalidSession
    ));
    let harness = Harness::new();
    {
        let mut state = harness.state.lock().unwrap();
        state.found = Some(receipt());
        state.change_on_find = true;
    }
    assert!(matches!(
        failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &signature()
        )),
        ApplicationError::InvalidSession
    ));
}

#[test]
fn receipt_reads_reauthenticate_and_reject_cross_account_or_withdrawn_current_results() {
    let harness = Harness::new();
    harness.state.lock().unwrap().found = Some(receipt());
    assert_eq!(harness.service().get("session", id()).unwrap(), receipt());
    let harness = Harness::new();
    {
        let mut state = harness.state.lock().unwrap();
        state.found = Some(receipt());
        state.change_on_find = true;
    }
    assert!(matches!(
        failure(harness.service().current("session")),
        ApplicationError::InvalidSession
    ));
    let harness = Harness::new();
    let mut wrong = receipt();
    let other = UserId::from_uuid(Uuid::from_u128(2));
    wrong.statement = domain::document_signing::SigningBindingStatement::new(
        SignerAccount::new(other, Role::Litigator, true, 9, 4).unwrap(),
        other,
        *statement().material(),
    )
    .unwrap();
    harness.state.lock().unwrap().found = Some(wrong);
    assert_eq!(
        kind(failure(harness.service().get("session", id()))),
        Error::Inconsistent
    );
    let harness = Harness::new();
    let mut retired = receipt();
    retired.withdrawn_at = Some(at(1001));
    harness.state.lock().unwrap().found = Some(retired);
    assert_eq!(
        kind(failure(harness.service().current("session"))),
        Error::Inconsistent
    );
}

#[test]
fn withdrawal_preserves_registration_and_needs_authority_but_no_live_credential() {
    let mut harness = Harness::new();
    {
        let mut state = harness.state.lock().unwrap();
        state.found = Some(receipt());
        state.context.trust = None;
        state.now = 5000;
    }
    harness
        .store
        .expect_withdraw()
        .times(1)
        .returning(|actor, binding, expected, at| {
            assert_eq!(actor, &principal());
            assert_eq!(binding, id());
            assert_eq!(expected, account());
            let mut value = receipt();
            value.withdrawn_at = Some(at);
            Ok(value)
        });
    let value = harness.service().withdraw("session", id()).unwrap();
    assert_eq!(value.statement, statement());
    assert_eq!(value.check, receipt().check);
    assert_eq!(value.withdrawn_at, Some(at(5000)));
}

#[test]
fn non_litigators_and_invalid_inputs_cannot_reach_the_binding_store() {
    for role in [Role::Owner, Role::Paralegal, Role::Client] {
        let harness = Harness::new();
        harness
            .state
            .lock()
            .unwrap()
            .principal
            .as_mut()
            .unwrap()
            .role = role;
        let state = harness.state.clone();
        assert!(matches!(
            failure(
                harness
                    .service()
                    .prepare("session", id(), &certificate().der)
            ),
            ApplicationError::PermissionDenied
        ));
        assert_eq!(state.lock().unwrap().calls, ["authenticate"]);
    }
    let harness = Harness::new();
    assert_eq!(
        kind(failure(harness.service().prepare(
            "session",
            Uuid::nil(),
            &certificate().der
        ))),
        Error::Input
    );
    let harness = Harness::new();
    assert_eq!(
        kind(failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &Signature::from_bytes(vec![1]).unwrap()
        ))),
        Error::Input
    );
}

#[test]
fn substituted_verifier_evidence_and_expiration_during_verification_never_commit() {
    let mut harness = Harness::new();
    harness.verify_with(|check, _| check.signature = Signature::from_bytes(vec![6; 384]).unwrap());
    assert_eq!(
        kind(failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &signature()
        ))),
        Error::Inconsistent
    );
    let mut harness = Harness::new();
    harness.verify_with(|_, state| state.lock().unwrap().now = 2001);
    assert_eq!(
        kind(failure(harness.service().register(
            "session",
            &statement(),
            &certificate().der,
            &signature()
        ))),
        Error::Credential(CredentialFailure::Expired)
    );
}
