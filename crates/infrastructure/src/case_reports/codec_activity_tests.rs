use super::*;
use application::cases::CaseStatusFilter;
use domain::identity::{Role, UserId};
use serde_json::json;

fn capture() -> CaseReportSnapshot {
    let at = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
    CaseReportSnapshot {
        report_id: CaseReportId::from_uuid(Uuid::from_u128(1)),
        requester: CaseReportRequester {
            principal: Principal {
                id: UserId::from_uuid(Uuid::from_u128(2)),
                email: "owner@example.test".into(),
                role: Role::Owner,
            },
            account_revision: 4,
            auth_generation: 2,
        },
        scope: CaseReportScope::Office,
        filters: CaseReportFilters {
            kind: CaseReportKind::CaseState,
            period_from: at,
            period_before: at + time::Duration::days(1),
            status: CaseStatusFilter::All,
            litigator: None,
        },
        checked_at: at,
        cases: vec![],
        workload: vec![],
        activity: None,
        digest: Sha256Digest::from_array([0; 32]),
    }
}

#[test]
fn legacy_storage_omits_new_discriminators_and_preserves_command_shape() {
    let value = capture();
    let command = CaseReportCommand {
        operation_id: CaseReportOperationId::from_uuid(value.report_id.as_uuid()),
        filters: value.filters.clone(),
    };
    let wire = codec::command(&command).unwrap();
    assert_eq!(
        wire,
        json!({"operation_id": command.operation_id.to_string(),
        "filters":{"from":"2025-01-01T00:00:00Z","before":"2025-01-02T00:00:00Z",
            "status":"all","litigator":null}})
    );
    assert_eq!(codec::read_command(wire).unwrap(), command);
    let bytes = codec::snapshot(&value).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire["version"], 1);
    assert!(wire.get("activity").is_none());
    assert!(wire["filters"].get("kind").is_none());
    assert_eq!(codec::read_snapshot(&bytes).unwrap(), value);
    assert_eq!(
        codec::snapshot(&codec::read_snapshot(&bytes).unwrap()).unwrap(),
        bytes
    );
}

#[test]
fn activity_storage_round_trips_without_reinterpreting_legacy_captures() {
    let mut value = capture();
    value.filters.kind = CaseReportKind::LitigatorActivity;
    value.activity = Some(CaseReportActivitySnapshot {
        actors: vec![],
        rows: vec![],
        documents_complete: false,
    });
    let bytes = codec::snapshot(&value).unwrap();
    assert_eq!(codec::read_snapshot(&bytes).unwrap(), value);
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(wire["version"], 2);
    assert_eq!(wire["filters"]["kind"], "litigator_activity");
    for mutation in 0..4 {
        let mut broken = wire.clone();
        match mutation {
            0 => broken["version"] = json!(1),
            1 => {
                broken.as_object_mut().unwrap().remove("activity");
            }
            2 => broken["filters"]["kind"] = json!("case_state"),
            _ => broken["activity"]["invented"] = json!(true),
        }
        assert!(
            codec::read_snapshot(&serde_json::to_vec(&broken).unwrap()).is_err(),
            "mutation {mutation}"
        );
    }
    let command = CaseReportCommand {
        operation_id: CaseReportOperationId::from_uuid(value.report_id.as_uuid()),
        filters: value.filters,
    };
    assert_eq!(
        codec::read_command(codec::command(&command).unwrap()).unwrap(),
        command
    );
}
