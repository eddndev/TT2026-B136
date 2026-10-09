use super::case_report_support::*;
use application::{case_reports::*, cases::CaseStatusFilter, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Role, UserId},
    DomainError,
};
use std::{io::Read, sync::Mutex};
use uuid::Uuid;

#[derive(Default)]
struct CapturedBytes(Mutex<Vec<u8>>);
impl DocumentHasher for CapturedBytes {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        *self.0.lock().unwrap() = bytes.to_vec();
        TestHasher.hash_bytes(bytes)
    }
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        Ok(self.hash_bytes(&bytes))
    }
}
impl CapturedBytes {
    fn hex(&self) -> String {
        self.0
            .lock()
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
fn user(id: u128) -> UserId {
    UserId::from_uuid(Uuid::from_u128(id))
}
fn state(role: Role) -> CaseReportSnapshot {
    let mut who = actor(role);
    who.id = user(1);
    let mut report = detail(&who, command());
    report.id = CaseReportId::from_uuid(Uuid::from_u128(3));
    let mut value = snapshot(&report);
    value.cases[0].case_id = CaseId::from_uuid(Uuid::from_u128(4));
    if role == Role::Owner {
        value.cases[0].assigned_litigators[0].user_id = user(5);
        value.workload[0].litigator.user_id = user(5);
    }
    value
}
fn activity(role: Role) -> CaseReportSnapshot {
    let mut value = state(role);
    value.filters.kind = CaseReportKind::LitigatorActivity;
    value.cases[0].created_at = value.filters.period_from - time::Duration::days(365);
    value.workload.clear();
    value.activity = Some(CaseReportActivitySnapshot {
        actors: vec![CaseReportLitigator {
            user_id: user(6),
            email: "former@example.test".into(),
        }],
        rows: vec![CaseReportActivityRow {
            case_id: value.cases[0].case_id,
            litigator_id: user(6),
            documents_uploaded: 2,
            procedural_activities: 3,
            deadlines_attended: 4,
        }],
        documents_complete: true,
    });
    value
}
fn multiple_rows() -> CaseReportSnapshot {
    let mut value = activity(Role::Owner);
    let mut second = value.cases[0].clone();
    second.case_id = CaseId::from_uuid(Uuid::from_u128(5));
    value.cases.push(second);
    let payload = value.activity.as_mut().unwrap();
    payload.actors.push(CaseReportLitigator {
        user_id: user(7),
        email: "other@example.test".into(),
    });
    let mut other_author = payload.rows[0].clone();
    other_author.litigator_id = user(7);
    payload.rows.push(other_author);
    let mut other_case = payload.rows[0].clone();
    other_case.case_id = value.cases[1].case_id;
    payload.rows.push(other_case);
    value
}
fn valid(mut value: CaseReportSnapshot) {
    value.digest = case_report_snapshot_digest(&TestHasher, &value).unwrap();
    validate_case_report_snapshot(&TestHasher, &value).unwrap();
}
fn rejected(value: &CaseReportSnapshot) {
    assert!(case_report_snapshot_digest(&TestHasher, value).is_err());
    assert!(validate_case_report_snapshot(&TestHasher, value).is_err());
}

#[test]
fn state_request_and_snapshot_preserve_historical_bytes_and_digests() {
    let value = state(Role::Owner);
    assert_eq!(value.filters.kind, CaseReportKind::CaseState);
    assert!(value.activity.is_none());
    let command = CaseReportCommand {
        operation_id: CaseReportOperationId::from_uuid(Uuid::from_u128(2)),
        filters: value.filters.clone(),
    };
    let hasher = CapturedBytes::default();
    let digest =
        case_report_request_digest(&hasher, &value.requester.principal, value.scope, &command)
            .unwrap();
    assert_eq!(
        digest.to_hex(),
        "d820611f4fdd60e6349e25de992561b0ffa398f8599c04473d0a942f064a510c"
    );
    assert_eq!(
        hasher.hex(),
        concat!(
            "000000000000001674742e636173652d7265706f72742e7265717565737400000000",
            "000000010000000000000000000000000000000100000000000000157265706f",
            "72746572406578616d706c652e7465737400000000000000056f776e6572000000",
            "000000000100000000000000000000000000000002000000000000000018f1ad",
            "0d8c720000000000000000000018fae27693b40000000000000000000000000000",
            "00000000",
        )
    );
    let digest = case_report_snapshot_digest(&hasher, &value).unwrap();
    assert_eq!(
        digest.to_hex(),
        "760b8c73f8f358f482bb106716938796bca0397aa3027536af2640c4f66d5c3e"
    );
    assert_eq!(
        hasher.hex(),
        concat!(
            "000000000000001774742e636173652d7265706f72742e736e617073686f74000000",
            "0000000001000000000000000000000000000000030000000000000000000000",
            "000000000100000000000000157265706f72746572406578616d706c652e746573",
            "7400000000000000056f776e657200000000000000010000000000000001000000",
            "0000000001000000000000000018f1ad0d8c720000000000000000000018fae276",
            "93b4000000000000000000000000000000000000000000000000000018fae276",
            "93b4000000000000000000010000000000000000000000000000000400000000",
            "0000000c43757272656e74206361736500000000000000055245462d3100000000",
            "0000000018fa93e202650000000000000000000661637469766500000000000000",
            "0000000000000000010000000000000000000000000000000500000000000000",
            "166c6974696761746f72406578616d706c652e7465737400000000000000010000",
            "000000000000000000000000000500000000000000166c6974696761746f724065",
            "78616d706c652e7465737400000000000000010000000000000000",
        )
    );
}

#[test]
fn request_kind_and_captured_activity_fields_change_the_digest() {
    let value = state(Role::Owner);
    let mut input = command();
    let original =
        case_report_request_digest(&TestHasher, &value.requester.principal, value.scope, &input)
            .unwrap();
    input.filters.kind = CaseReportKind::LitigatorActivity;
    assert_ne!(
        original,
        case_report_request_digest(&TestHasher, &value.requester.principal, value.scope, &input,)
            .unwrap()
    );
    let original = activity(Role::Owner);
    let digest = case_report_snapshot_digest(&TestHasher, &original).unwrap();
    for mutation in 0..5 {
        let mut changed = original.clone();
        let payload = changed.activity.as_mut().unwrap();
        match mutation {
            0 => payload.rows[0].documents_uploaded += 1,
            1 => payload.rows[0].procedural_activities += 1,
            2 => payload.rows[0].deadlines_attended += 1,
            3 => payload.documents_complete = false,
            _ => payload.actors[0].email = "renamed@example.test".into(),
        }
        assert_ne!(
            digest,
            case_report_snapshot_digest(&TestHasher, &changed).unwrap()
        );
    }
}

#[test]
fn activity_period_and_author_are_independent_of_creation_and_current_assignment() {
    let mut value = activity(Role::Litigator);
    value.filters.litigator = Some(user(6));
    valid(value.clone());
    value.filters.litigator = Some(user(7));
    rejected(&value);
    value.filters.litigator = None;
    value.cases[0].assigned_litigators[0].user_id = user(8);
    rejected(&value);
    value.requester.principal.role = Role::Owner;
    value.scope = CaseReportScope::Office;
    valid(value.clone());
    value.filters.status = CaseStatusFilter::Closed;
    rejected(&value);
    value.filters.status = CaseStatusFilter::All;
    value.cases[0].created_at = value.checked_at + time::Duration::seconds(1);
    rejected(&value);
}

#[test]
fn rows_require_captured_case_and_actor_with_unique_sorted_pairs() {
    let original = multiple_rows();
    valid(original.clone());
    for mutation in 0..7 {
        let mut value = original.clone();
        let payload = value.activity.as_mut().unwrap();
        match mutation {
            0 => payload.rows.push(payload.rows[0].clone()),
            1 => payload.rows.swap(0, 1),
            2 => payload.rows.swap(0, 2),
            3 => payload.rows[0].case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            4 => payload.rows[0].litigator_id = user(99),
            5 => payload.actors.reverse(),
            _ => payload.actors.push(payload.actors[0].clone()),
        }
        rejected(&value);
    }
}

#[test]
fn activity_payload_is_required_only_for_its_own_kind() {
    let mut value = activity(Role::Owner);
    value.filters.kind = CaseReportKind::CaseState;
    value.cases[0].created_at = now() - time::Duration::days(1);
    value.workload = state(Role::Owner).workload;
    rejected(&value);
    value.activity = None;
    valid(value.clone());
    value.filters.kind = CaseReportKind::LitigatorActivity;
    value.workload.clear();
    rejected(&value);
}

#[test]
fn activity_rejects_counter_total_overflow_for_each_metric() {
    for metric in 0..3 {
        for second in [1, 2] {
            let mut value = multiple_rows();
            let payload = value.activity.as_mut().unwrap();
            for row in &mut payload.rows {
                row.documents_uploaded = 0;
                row.procedural_activities = 0;
                row.deadlines_attended = 0;
            }
            for (index, amount) in [(0, u64::MAX), (second, 1)] {
                let row = &mut payload.rows[index];
                match metric {
                    0 => row.documents_uploaded = amount,
                    1 => row.procedural_activities = amount,
                    _ => row.deadlines_attended = amount,
                }
            }
            rejected(&value);
        }
    }
}

#[test]
fn activity_reuses_bounded_case_actor_row_and_snapshot_capacities() {
    assert_eq!(MAX_REPORT_CASES, 1000);
    assert_eq!(MAX_REPORT_WORKLOAD, 1000);
    assert_eq!(MAX_REPORT_ASSIGNMENTS, 10000);
    assert_eq!(MAX_REPORT_SNAPSHOT_BYTES, 8 * 1024 * 1024);
    for dimension in 0..3 {
        let mut value = activity(Role::Owner);
        let payload = value.activity.as_mut().unwrap();
        match dimension {
            0 => value.cases = vec![value.cases[0].clone(); MAX_REPORT_CASES + 1],
            1 => payload.actors = vec![payload.actors[0].clone(); MAX_REPORT_WORKLOAD + 1],
            _ => payload.rows = vec![payload.rows[0].clone(); MAX_REPORT_ASSIGNMENTS + 1],
        }
        assert!(matches!(
            case_report_snapshot_digest(&TestHasher, &value),
            Err(ApplicationError::CaseReport(
                CaseReportError::CapacityExceeded
            ))
        ));
    }
}
