#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_input_service_support;
mod deadline_input_support;

use application::{deadline_inputs::*, ApplicationError};
use case_support::MockIdentity;
use deadline_input_service_support::*;
use deadline_input_support::{material, request, resolution, unknown};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{ArithmeticBlock, ArithmeticOutcome, ArithmeticRule},
    deadline_triggers::{TriggerBlock, TriggerOutcome},
    identity::{Role, UserId},
    procedural_time::DeclaredProceduralPrecision,
};
use mockall::Sequence;
use std::num::NonZeroU32;
use uuid::Uuid;

#[test]
fn four_role_policy_is_checked_before_any_source_load() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let (request, material) = unknown();
        let actor = principal(role);
        let mut identity = MockIdentity::new();
        let mut store = MockStore::new();
        let mut sequence = Sequence::new();
        expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
        if role == Role::Client {
            store.expect_load().times(0);
            assert!(matches!(
                service(store, identity).prepare("session", request),
                Err(ApplicationError::PermissionDenied)
            ));
        } else {
            expect_load(
                &mut store,
                &mut sequence,
                actor.id,
                request.clone(),
                Ok(material),
            );
            expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
            let prepared = service(store, identity)
                .prepare("session", request)
                .unwrap();
            assert_eq!(prepared.actor(), actor.id);
        }
    }
}

#[test]
fn invalid_session_never_touches_the_store() {
    let (request, _) = unknown();
    let mut identity = MockIdentity::new();
    let mut store = MockStore::new();
    let mut sequence = Sequence::new();
    expect_identity(
        &mut identity,
        &mut sequence,
        Err(ApplicationError::InvalidSession),
    );
    store.expect_load().times(0);
    assert!(matches!(
        service(store, identity).prepare("session", request),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn store_failures_propagate_without_returning_a_preparation() {
    for failure in [
        ApplicationError::Port("load sentinel".into()),
        ApplicationError::PermissionDenied,
    ] {
        let (request, _) = unknown();
        let actor = principal(Role::Owner);
        let mut identity = MockIdentity::new();
        let mut store = MockStore::new();
        let mut sequence = Sequence::new();
        let is_port = matches!(&failure, ApplicationError::Port(_));
        expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
        expect_load(
            &mut store,
            &mut sequence,
            actor.id,
            request.clone(),
            Err(failure),
        );
        let result = service(store, identity).prepare("session", request);
        if is_port {
            assert!(
                matches!(result, Err(ApplicationError::Port(message)) if message == "load sentinel")
            );
        } else {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
    }
}

#[test]
fn a_token_revoked_after_loading_cannot_return_loaded_material() {
    let (request, material) = unknown();
    let actor = principal(Role::Litigator);
    let mut identity = MockIdentity::new();
    let mut store = MockStore::new();
    let mut sequence = Sequence::new();
    expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
    expect_load(
        &mut store,
        &mut sequence,
        actor.id,
        request.clone(),
        Ok(material),
    );
    expect_identity(
        &mut identity,
        &mut sequence,
        Err(ApplicationError::InvalidSession),
    );
    assert!(matches!(
        service(store, identity).prepare("session", request),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn another_allowed_actor_or_role_cannot_receive_the_original_preparation() {
    let first = principal(Role::Owner);
    let mut other_actor = first.clone();
    other_actor.id = UserId::from_uuid(Uuid::from_u128(999));
    let mut litigator = first.clone();
    litigator.role = Role::Litigator;
    let mut paralegal = first.clone();
    paralegal.role = Role::Paralegal;
    for changed in [other_actor, litigator, paralegal] {
        let (request, material) = unknown();
        let mut identity = MockIdentity::new();
        let mut store = MockStore::new();
        let mut sequence = Sequence::new();
        expect_identity(&mut identity, &mut sequence, Ok(first.clone()));
        expect_load(
            &mut store,
            &mut sequence,
            first.id,
            request.clone(),
            Ok(material),
        );
        expect_identity(&mut identity, &mut sequence, Ok(changed));
        assert!(matches!(
            service(store, identity).prepare("session", request),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn a_new_client_role_is_denied_before_principal_equality_is_considered() {
    for same_actor in [true, false] {
        let (request, material) = unknown();
        let actor = principal(Role::Owner);
        let mut changed = actor.clone();
        changed.role = Role::Client;
        if !same_actor {
            changed.id = UserId::from_uuid(Uuid::from_u128(999));
        }
        let mut identity = MockIdentity::new();
        let mut store = MockStore::new();
        let mut sequence = Sequence::new();
        expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
        expect_load(
            &mut store,
            &mut sequence,
            actor.id,
            request.clone(),
            Ok(material),
        );
        expect_identity(&mut identity, &mut sequence, Ok(changed));
        assert!(matches!(
            service(store, identity).prepare("session", request),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn prepare_preserves_request_material_actor_and_exact_calculated_date() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(3, false, "2026-03-02")));
    let request = request(&source);
    let material = material(source);
    let actor = principal(Role::Paralegal);
    let mut identity = MockIdentity::new();
    let mut store = MockStore::new();
    let mut sequence = Sequence::new();
    expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
    expect_load(
        &mut store,
        &mut sequence,
        actor.id,
        request.clone(),
        Ok(material.clone()),
    );
    expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
    let prepared = service(store, identity)
        .prepare("session", request.clone())
        .unwrap();
    assert_eq!(prepared.actor(), actor.id);
    assert_eq!(prepared.request(), &request);
    assert_eq!(prepared.material(), &material);
    assert_eq!(prepared.calculation().rule(), request.rule);
    assert_eq!(
        prepared.calculation().trigger().selection(),
        &request.trigger
    );
    let arithmetic = prepared.calculation().arithmetic().unwrap();
    assert_eq!(
        arithmetic.anchor(),
        deadline_input_support::date("2026-03-02")
    );
    assert_eq!(
        arithmetic.outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2026-03-02".parse().unwrap(),
        }
    );
}

#[test]
fn unknown_source_is_loaded_and_returned_as_a_valid_blocked_calculation() {
    let (request, material) = unknown();
    let actor = principal(Role::Owner);
    let mut identity = MockIdentity::new();
    let mut store = MockStore::new();
    let mut sequence = Sequence::new();
    expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
    expect_load(
        &mut store,
        &mut sequence,
        actor.id,
        request.clone(),
        Ok(material.clone()),
    );
    expect_identity(&mut identity, &mut sequence, Ok(actor));
    let prepared = service(store, identity)
        .prepare("session", request.clone())
        .unwrap();
    assert_eq!(prepared.request(), &request);
    assert_eq!(prepared.material(), &material);
    assert_eq!(
        prepared.calculation().trigger().outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
    assert!(prepared.calculation().arithmetic().is_none());
}

#[test]
fn insufficient_temporal_precision_remains_an_arithmetic_block() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(3, false, "2026-03-02")));
    let mut request = request(&source);
    request.rule = ArithmeticRule::ElapsedHours {
        quantity: NonZeroU32::new(2).unwrap(),
    };
    let material = material(source);
    let actor = principal(Role::Litigator);
    let mut identity = MockIdentity::new();
    let mut store = MockStore::new();
    let mut sequence = Sequence::new();
    expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
    expect_load(
        &mut store,
        &mut sequence,
        actor.id,
        request.clone(),
        Ok(material),
    );
    expect_identity(&mut identity, &mut sequence, Ok(actor));
    let prepared = service(store, identity)
        .prepare("session", request)
        .unwrap();
    assert_eq!(
        prepared.calculation().arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::Blocked(ArithmeticBlock::InsufficientPrecision {
            observed: DeclaredProceduralPrecision::Date,
        })
    );
}

#[test]
fn inconsistent_loaded_material_is_rejected_by_the_service_checker() {
    let (request, mut material) = unknown();
    material.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    let actor = principal(Role::Owner);
    let mut identity = MockIdentity::new();
    let mut store = MockStore::new();
    let mut sequence = Sequence::new();
    expect_identity(&mut identity, &mut sequence, Ok(actor.clone()));
    expect_load(
        &mut store,
        &mut sequence,
        actor.id,
        request.clone(),
        Ok(material),
    );
    assert!(matches!(
        service(store, identity).prepare("session", request),
        Err(ApplicationError::DeadlineInput(
            DeadlineInputError::Inconsistent(_)
        ))
    ));
}
