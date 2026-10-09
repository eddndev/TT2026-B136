#[path = "case_report_activity_model.rs"]
mod activity;
mod case_report_support;
#[allow(dead_code)]
mod case_support;
use application::case_reports::*;
use case_report_support::*;
use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
};

#[test]
fn request_digest_binds_operation_identity_scope_and_every_filter() {
    let who = actor(Role::Owner);
    let original = command();
    let expected =
        case_report_request_digest(&TestHasher, &who, CaseReportScope::Office, &original).unwrap();
    assert_eq!(
        expected,
        case_report_request_digest(&TestHasher, &who, CaseReportScope::Office, &original).unwrap()
    );
    for mutation in 0..7 {
        let mut input = original.clone();
        let mut actor = who.clone();
        match mutation {
            0 => input.operation_id = CaseReportOperationId::new(),
            1 => input.filters.period_from -= time::Duration::days(1),
            2 => input.filters.period_before -= time::Duration::seconds(1),
            3 => input.filters.status = application::cases::CaseStatusFilter::Active,
            4 => input.filters.litigator = Some(UserId::new()),
            5 => actor.id = UserId::new(),
            _ => actor.email = "other@example.test".into(),
        }
        assert_ne!(
            expected,
            case_report_request_digest(&TestHasher, &actor, CaseReportScope::Office, &input)
                .unwrap()
        );
    }
    assert!(case_report_request_digest(
        &TestHasher,
        &who,
        CaseReportScope::AssignedCases,
        &original
    )
    .is_err());
}

#[test]
fn canonical_snapshot_binds_capture_and_ignores_only_its_own_digest() {
    let original = snapshot(&detail(&actor(Role::Owner), command()));
    validate_case_report_snapshot(&TestHasher, &original).unwrap();
    let mut own_digest = original.clone();
    own_digest.digest = Sha256Digest::from_array([99; 32]);
    assert_eq!(
        original.digest,
        case_report_snapshot_digest(&TestHasher, &own_digest).unwrap()
    );
    for mutation in 0..9 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.report_id = CaseReportId::new(),
            1 => changed.checked_at -= time::Duration::seconds(1),
            2 => changed.cases[0].title = "Changed title".into(),
            3 => changed.cases[0].reference = "REF-2".into(),
            4 => changed.cases[0].created_at -= time::Duration::seconds(1),
            5 => changed.requester.account_revision += 1,
            6 => changed.requester.auth_generation += 1,
            7 => {
                changed.cases[0].assigned_litigators[0].email = "changed@example.test".into();
                changed.workload[0].litigator.email = "changed@example.test".into();
            }
            _ => {
                changed.cases[0].status =
                    domain::case_administration::CaseAdministrativeStatus::Closed;
                changed.workload[0].active_cases = 0;
                changed.workload[0].closed_cases = 1;
            }
        }
        assert_ne!(
            original.digest,
            case_report_snapshot_digest(&TestHasher, &changed).unwrap()
        );
    }
}

#[test]
fn snapshot_validation_rejects_omitted_workload_duplicates_and_outside_filters() {
    let original = snapshot(&detail(&actor(Role::Owner), command()));
    for mutation in 0..6 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.workload.clear(),
            1 => changed.cases.push(changed.cases[0].clone()),
            2 => {
                let duplicate = changed.cases[0].assigned_litigators[0].clone();
                changed.cases[0].assigned_litigators.push(duplicate);
            }
            3 => changed.cases[0].created_at = changed.filters.period_before,
            4 => changed.workload[0].active_cases += 1,
            _ => changed.requester.auth_generation = i64::MAX as u64 + 1,
        }
        assert!(case_report_snapshot_digest(&TestHasher, &changed).is_err());
        assert!(validate_case_report_snapshot(&TestHasher, &changed).is_err());
    }
    let mut tampered = original;
    tampered.digest = Sha256Digest::from_array([77; 32]);
    inconsistent(validate_case_report_snapshot(&TestHasher, &tampered));
}

#[test]
fn assignment_scope_requires_requester_on_every_case_and_caps_before_partial_totals() {
    let original = snapshot(&detail(&actor(Role::Litigator), command()));
    validate_case_report_snapshot(&TestHasher, &original).unwrap();
    let mut foreign = original.clone();
    foreign.cases[0].assigned_litigators[0].user_id = UserId::new();
    foreign.workload[0].litigator.user_id = foreign.cases[0].assigned_litigators[0].user_id;
    assert!(case_report_snapshot_digest(&TestHasher, &foreign).is_err());
    assert!(validate_case_report_snapshot(&TestHasher, &foreign).is_err());
    let mut excessive = original;
    excessive.cases = vec![excessive.cases[0].clone(); MAX_REPORT_CASES + 1];
    assert!(matches!(
        case_report_snapshot_digest(&TestHasher, &excessive),
        Err(application::ApplicationError::CaseReport(
            CaseReportError::CapacityExceeded
        ))
    ));
}

#[test]
fn zero_account_stamps_are_valid_initial_database_generations() {
    let mut captured = snapshot(&detail(&actor(Role::Owner), command()));
    captured.requester.account_revision = 0;
    captured.requester.auth_generation = 0;
    captured.digest = case_report_snapshot_digest(&TestHasher, &captured).unwrap();
    validate_case_report_snapshot(&TestHasher, &captured).unwrap();
    captured.requester.auth_generation = i64::MAX as u64 + 1;
    assert!(case_report_snapshot_digest(&TestHasher, &captured).is_err());
}
