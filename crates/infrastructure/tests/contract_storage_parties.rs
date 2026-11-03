//! Who may be a party of a contract request in PostgreSQL: an invited Client,
//! an Owner or member Litigator firm signer, live bindings, an open case, and
//! a counter-signature that does not depend on the Client binding:
//! docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{
    cases::{CaseAdministrativeStatus, CaseRepository as _, CaseRevisionExpectation},
    document_signing::SigningBindingStore as _,
};
use contract_storage_support::{opens, rejection, Harness};
use domain::clock::OffsetDateTime;
use std::sync::Arc;

fn denied(message: &str) -> (String, String) {
    ("42501".into(), message.into())
}

/// Withdraws the live binding of the Client or of the firm signer at `when`.
fn withdraw(test: &mut Harness, client: bool, when: i64) {
    test.individual.clock.set(when);
    let store = signing_binding_support::store(&test.individual.db, &test.individual.clock);
    let (principal, binding) = if client {
        (test.client.clone(), test.client_binding.clone())
    } else {
        (test.firm(), test.firm_binding.clone())
    };
    let context = store.load_context(principal.id).unwrap();
    let at = OffsetDateTime::from_unix_timestamp(when).unwrap();
    store
        .withdraw(
            &principal,
            binding.statement.material().binding(),
            context.account,
            at,
        )
        .unwrap();
}

#[test]
fn only_a_client_who_accepted_a_client_invitation_is_eligible() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let client = test.client.id.as_uuid();
    test.db()
        .admin
        .execute(
            "UPDATE member_invitations SET role='paralegal' WHERE id=(SELECT invitation_id
            FROM member_invitation_completions WHERE user_id=$1)",
            &[&client],
        )
        .unwrap();
    let request = test.prepare(test.now() + 60, 7);
    assert_eq!(
        rejection(test.insert_request(&request)),
        denied("contract client is not eligible")
    );
}

#[test]
fn a_firm_signer_must_be_an_owner_or_a_litigator_member_of_the_case() {
    for change in [
        "DELETE FROM case_memberships WHERE user_id=$1",
        "UPDATE users SET role='paralegal',revision=revision+1,auth_generation=auth_generation+1
        WHERE id=$1",
    ] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let firm = test.firm().id.as_uuid();
        test.db().admin.execute(change, &[&firm]).unwrap();
        let request = test.prepare(test.now() + 60, 7);
        assert_eq!(
            rejection(test.insert_request(&request)),
            denied("contract firm signer is not eligible"),
            "{change}"
        );
    }
}

#[test]
fn a_request_needs_both_bindings_alive() {
    for client in [true, false] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let now = test.now();
        withdraw(&mut test, client, now);
        let request = test.prepare(now + 60, 7);
        assert_eq!(
            rejection(test.insert_request(&request)),
            ("23514".into(), "contract party binding is inactive".into())
        );
    }
}

#[test]
fn the_counter_signature_survives_a_withdrawn_client_binding_but_not_its_own() {
    for client in [true, false] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let request = test.prepare(test.now() + 60, 7);
        test.insert_request(&request).unwrap();
        test.sign_client(&request, request.created() + 60).unwrap();
        withdraw(&mut test, client, request.created() + 90);
        let result = test.sign_firm(&request, request.created() + 120);
        if client {
            result.unwrap();
            assert!(opens(&test));
        } else {
            assert_eq!(
                rejection(result),
                (
                    "23514".into(),
                    "contract signing binding is inactive or inconsistent".into()
                )
            );
        }
    }
}

#[test]
fn a_closed_case_blocks_requests_and_signatures_but_still_records_expiry() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 1);
    test.insert_request(&request).unwrap();
    let cases = infrastructure::PostgresCaseRepository::connect(
        &test.individual.db.admin_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    let (owner, case) = (test.individual.db.owner, test.individual.db.case);
    let at = OffsetDateTime::from_unix_timestamp(request.created() + 30).unwrap();
    cases
        .change_administrative_status(
            owner,
            case,
            CaseRevisionExpectation::Unrevised,
            CaseAdministrativeStatus::Closed,
            at,
        )
        .unwrap();
    assert_eq!(
        rejection(test.sign_client(&request, request.created() + 60)),
        denied("contract actor is not currently authorized")
    );
    test.close(&request, "expired", None, None, request.expires())
        .unwrap();
    let later = test.prepare(request.expires() + 1, 7);
    assert_eq!(
        rejection(test.insert_request(&later)),
        denied("contract case is closed")
    );
}
