#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
#[path = "procedural_fact_hearing_support/mod.rs"]
mod hearing_source_support;
mod procedural_fact_service_boundary_support;
#[allow(dead_code)]
mod procedural_fact_service_support;
#[allow(dead_code)]
#[path = "procedural_fact_service_support/store.rs"]
mod store;
use application::{procedural_facts::*, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use procedural_fact_service_boundary_support::*;
use procedural_fact_service_support::*;
use std::sync::{atomic::Ordering, Arc};
use store::*;

#[test]
fn notification_admits_two_versions_in_one_batch_without_expanding_parent_support() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let parent = parent(actor.id, case);
    let first = record();
    let second = second_version(&first);
    let records = vec![first, second];
    let values = notice(&parent, &records);
    let expected: Vec<_> = records.iter().map(support).collect();
    let selected = parent_ref(&parent);
    let historical = parent.admitted_support.as_ref().unwrap().reference;
    let mut prep = preparation(case);
    prep.source_material.resolution = Some(Box::new(parent));
    prep.records = records.into_iter().rev().collect();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, clock) = service(store, identity, validator.clone());
    let draft = service
        .prepare("session", case, notice_command(values))
        .unwrap();
    assert_eq!(validator.calls.load(Ordering::SeqCst), 1);
    assert_eq!(validator.items.load(Ordering::SeqCst), 2);
    assert_eq!(draft.sources.direct_supports, expected);
    assert_eq!(
        draft.sources.resolved.resolution.unwrap().reference,
        selected
    );
    assert!(draft
        .sources
        .direct_supports
        .iter()
        .all(|source| source.reference != historical));
    assert_eq!(clock.calls(), 0);
}
#[test]
fn shared_direct_version_is_admitted_once_without_losing_two_functions() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let parent = parent(actor.id, case);
    let record = record();
    let values = notice(&parent, &[record.clone(), record.clone()]);
    let mut prep = preparation(case);
    prep.source_material.resolution = Some(Box::new(parent));
    prep.records = vec![record];
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    let draft = service
        .prepare("session", case, notice_command(values.clone()))
        .unwrap();
    assert_eq!(
        draft.values,
        ProceduralFactValues::Notification(Box::new(values))
    );
    assert_eq!(validator.calls.load(Ordering::SeqCst), 1);
    assert_eq!(validator.items.load(Ordering::SeqCst), 1);
    assert_eq!(draft.sources.direct_supports.len(), 1);
}
#[test]
fn changed_actor_or_revoked_write_role_after_admission_never_commits() {
    for changed_actor in [false, true] {
        let validator = Arc::new(Validator::default());
        let observed = validator.clone();
        let mut identity = MockIdentity::new();
        let actor = application::identity::Principal {
            id: UserId::new(),
            email: "actor@example.test".into(),
            role: Role::Owner,
        };
        let actor_id = actor.id;
        let first_actor = actor.clone();
        let mut sequence = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(first_actor));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                assert_eq!(observed.calls.load(Ordering::SeqCst), 1);
                Ok(application::identity::Principal {
                    id: if changed_actor {
                        UserId::new()
                    } else {
                        actor.id
                    },
                    role: if changed_actor {
                        Role::Owner
                    } else {
                        Role::Paralegal
                    },
                    ..actor
                })
            });
        let case = CaseId::new();
        let record = record();
        let values = with_support(&record);
        let command = record_command(values.clone());
        let mut sources = empty();
        sources.direct_supports.push(support(&record));
        let expected = detail(actor_id, case, &command, values, sources);
        let mut prep = preparation(case);
        prep.records = vec![record];
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, validator);
        let result = service.submit(
            "session",
            case,
            command,
            expected.snapshot.metadata().receipt.submission_digest,
        );
        assert!(
            matches!(result, Err(ApplicationError::InvalidSession)) && changed_actor
                || matches!(result, Err(ApplicationError::PermissionDenied)) && !changed_actor
        );
    }
}
#[test]
fn withdrawal_preserves_nonempty_historical_support_without_admission() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let record = record();
    let values = with_support(&record);
    let mut sources = empty();
    sources.direct_supports.push(support(&record));
    let base = detail(
        actor.id,
        case,
        &record_command(values.clone()),
        values,
        sources,
    );
    let command = withdrawal(&base);
    let expected = base.clone();
    let mut prep = preparation(case);
    prep.base = Some(base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    let draft = service.prepare("session", case, command).unwrap();
    assert_eq!(draft.values, expected.snapshot.values());
    assert_eq!(draft.sources, expected.sources);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn withdrawal_rejects_extra_records_and_material_without_admitting_them() {
    for extra_record in [false, true] {
        let (identity, actor) = identity(Role::Owner, 1);
        let case = CaseId::new();
        let base = detail(actor.id, case, &command(), values(), empty());
        let command = withdrawal(&base);
        let mut prep = preparation(case);
        prep.base = Some(base);
        if extra_record {
            prep.records.push(record());
        } else {
            prep.source_material.resolution = Some(Box::new(parent(actor.id, case)));
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(prep));
        let validator = Arc::new(Validator::default());
        let (service, _) = service(store, identity, validator.clone());
        inconsistent(service.prepare("session", case, command));
        assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn correction_retains_the_exact_parent_projection_and_captured_digest() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let (parent, values, base) = correction_base(actor.id, case);
    let command = notice_correction(&base, values);
    let expected = base.sources.clone();
    let mut prep = preparation(case);
    prep.base = Some(base);
    prep.source_material.resolution = Some(Box::new(parent));
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    assert_eq!(
        service.prepare("session", case, command).unwrap().sources,
        expected
    );
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn correction_rejects_a_self_consistent_substitution_of_a_retained_reference() {
    let (identity, actor) = identity(Role::Owner, 1);
    let case = CaseId::new();
    let (mut parent, values, base) = correction_base(actor.id, case);
    let command = notice_correction(&base, values);
    replace_parent_summary(&mut parent, actor.id, case);
    let mut prep = preparation(case);
    prep.base = Some(base);
    prep.source_material.resolution = Some(Box::new(parent));
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    inconsistent(service.prepare("session", case, command));
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}
#[test]
fn closed_case_stale_revision_and_altered_base_are_rejected_before_admission() {
    for mode in 0..3 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case = CaseId::new();
        let record = record();
        let values = with_support(&record);
        let mut sources = empty();
        sources.direct_supports.push(support(&record));
        let mut base = detail(
            actor.id,
            case,
            &record_command(values.clone()),
            values.clone(),
            sources,
        );
        let FactTarget::Resolution(id) = base.snapshot.target() else {
            unreachable!()
        };
        let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            id,
            FactChange::correct(
                FactRevision::new(if mode == 1 { 2 } else { 1 }).unwrap(),
                values,
                text("Correction"),
            ),
        ));
        if mode == 2 {
            snapshot_mut(&mut base).metadata.receipt.submission_digest = digest(77);
        }
        let mut prep = preparation(case);
        prep.base = Some(base);
        prep.records = vec![record];
        if mode == 0 {
            prep.observed_administration = closed(case, actor.id);
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(prep));
        let validator = Arc::new(Validator::default());
        let (service, _) = service(store, identity, validator.clone());
        let result = service.prepare("session", case, command);
        match mode {
            0 => assert!(matches!(result, Err(ApplicationError::CaseClosed))),
            1 => assert!(matches!(
                result,
                Err(ApplicationError::ProceduralFact(
                    ProceduralFactError::RevisionConflict
                ))
            )),
            _ => inconsistent(result),
        }
        assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn committed_receipt_must_match_the_submitted_operation_case_and_actor() {
    for mode in 0..3 {
        let (identity, actor) = identity(Role::Owner, 2);
        let case = CaseId::new();
        let command = command();
        let expected = detail(actor.id, case, &command, values(), empty())
            .snapshot
            .metadata()
            .receipt
            .submission_digest;
        let FactTarget::Resolution(id) = command.target() else {
            unreachable!()
        };
        let returned_command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
            if mode == 0 {
                FactOperationId::new()
            } else {
                command.operation_id()
            },
            id,
            FactChange::record(values()),
        ));
        let result = detail(
            if mode == 2 { UserId::new() } else { actor.id },
            if mode == 1 { CaseId::new() } else { case },
            &returned_command,
            values(),
            empty(),
        );
        fact_receipt_matches(hasher().as_ref(), &result).unwrap();
        let prep = preparation(case);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .return_once(move |_, _, _, _| Ok(prep));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(result));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        inconsistent(service.submit("session", case, command, expected));
    }
}
#[test]
fn future_declared_date_is_preserved_without_a_capture_clock_policy() {
    let (identity, _) = identity(Role::Owner, 2);
    let case = CaseId::new();
    let values = future_values();
    let expected = values.issued_at();
    let command = record_command(values);
    let prep = preparation(case);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, clock) = service(store, identity, Arc::new(Validator::default()));
    let draft = service.prepare("session", case, command).unwrap();
    let ProceduralFactValues::Resolution(values) = draft.values else {
        unreachable!()
    };
    assert_eq!(values.issued_at(), expected);
    assert_eq!(clock.calls(), 0);
}
#[test]
fn missing_selected_parent_prevents_admission_of_valid_direct_records() {
    let (identity, actor) = identity(Role::Owner, 1);
    let case = CaseId::new();
    let parent = parent(actor.id, case);
    let record = record();
    let values = notice(&parent, std::slice::from_ref(&record));
    let mut prep = preparation(case);
    prep.records = vec![record];
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    inconsistent(service.prepare("session", case, notice_command(values)));
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}
