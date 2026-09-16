use super::{projection, response};
#[path = "response_test_support.rs"]
mod support;
use application::{cases::*, procedural_facts::*};
use axum::{http::StatusCode, response::IntoResponse};
use domain::{cases::CaseId, procedural_time::DeclaredProceduralTime};
use serde_json::{json, Value};
use support::*;

fn internal(result: Result<Value, crate::error::ApiError>) {
    assert_eq!(
        result.unwrap_err().into_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}
fn project(row: FactDetail) -> Result<Value, crate::error::ApiError> {
    let case = row.snapshot.case_id();
    let target = row.snapshot.target();
    projection::detail(row, case, target, None)
}
#[test]
fn unrevised_capture_preserves_metadata_without_inventing_provenance() {
    let row = detail();
    let value = project(row).unwrap();
    assert_eq!(value["family"], "resolution");
    assert!(value.get("resolution_id").is_none());
    assert_eq!(
        value["recorded_administration"],
        json!({"kind":"unrevised","title":"Case title","reference":"REF-1","status":"active"})
    );
    assert_eq!(value["receipt"]["sources_digest"], digest(2).to_hex());
    assert_eq!(value["sources"]["resolution"], Value::Null);
    assert_eq!(value["values"]["issued_at"], json!({"precision":"unknown"}));
}
#[test]
fn recorded_capture_keeps_its_exact_author_time_and_positive_revision() {
    let mut row = detail();
    let case = row.snapshot.case_id();
    let old = row.snapshot.metadata().clone();
    meta(&mut row).recorded_administration =
        CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
            case_id: case,
            revision: CaseRevision::new(3).unwrap(),
            values: administration().values(),
            values_digest: digest(9),
            changed_at: old.recorded_at,
            changed_by: old.recorded_by.clone(),
        }));
    let value = project(row).unwrap();
    let admin = &value["recorded_administration"];
    assert_eq!(admin["kind"], "recorded");
    assert_eq!(admin["revision"], 3);
    assert_eq!(admin["values_digest"], digest(9).to_hex());
    assert_eq!(admin["changed_by"]["email"], old.recorded_by.email);
    assert_eq!(admin["changed_at"], "2023-11-14T22:13:20Z");
}
#[test]
fn detail_rejects_foreign_case_family_parent_and_exact_revision() {
    let row = detail();
    let case = row.snapshot.case_id();
    let target = row.snapshot.target();
    internal(projection::detail(row.clone(), CaseId::new(), target, None));
    internal(projection::detail(
        row.clone(),
        case,
        FactTarget::Notification {
            id: NotificationId::new(),
            resolution_id: ResolutionId::new(),
        },
        None,
    ));
    internal(projection::detail(
        row,
        case,
        target,
        Some(FactRevision::new(2).unwrap()),
    ));
    let row = notification();
    let case = row.snapshot.case_id();
    let FactTarget::Notification { id, .. } = row.snapshot.target() else {
        unreachable!()
    };
    internal(projection::detail(
        row,
        case,
        FactTarget::Notification {
            id,
            resolution_id: ResolutionId::new(),
        },
        None,
    ));
}
#[test]
fn malformed_receipt_transition_or_exhaustion_is_an_internal_error() {
    for mutation in 0..5 {
        let mut row = detail();
        let metadata = meta(&mut row);
        match mutation {
            0 => metadata.receipt.expected_revision = u32::MAX,
            1 => metadata.receipt.action = FactAction::Correct,
            2 => metadata.status = FactStatus::Withdrawn,
            3 => metadata.reason = Some(text("Unexpected reason")),
            _ => metadata.revision = FactRevision::new(2).unwrap(),
        }
        internal(project(row));
    }
}
#[test]
fn recorded_administration_must_belong_to_case_and_have_been_active() {
    for foreign in [true, false] {
        let mut row = detail();
        let mut values = administration().values();
        if !foreign {
            values = values.with_status(CaseAdministrativeStatus::Closed);
        }
        meta(&mut row).recorded_administration =
            CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
                case_id: if foreign {
                    CaseId::new()
                } else {
                    row.snapshot.case_id()
                },
                revision: CaseRevision::FIRST,
                values,
                values_digest: digest(9),
                changed_at: row.snapshot.metadata().recorded_at,
                changed_by: row.snapshot.metadata().recorded_by.clone(),
            }));
        internal(project(row));
    }
}
#[test]
fn historical_parent_is_projected_and_unrecorded_received_time_stays_distinct() {
    let row = notification();
    let parent = row.sources.resolved.resolution.unwrap();
    let value = project(row).unwrap();
    assert_eq!(value["resolution_id"], parent.reference.id.to_string());
    assert_eq!(value["sources"]["resolution"]["status"], "withdrawn");
    assert_eq!(value["sources"]["resolution"]["revision"], 1);
    assert_eq!(
        value["values"]["received_at"],
        json!({"precision":"unknown"})
    );
}
#[test]
fn source_selection_missing_foreign_or_extra_is_not_serialized() {
    let mut row = notification();
    row.sources.resolved.resolution = None;
    row.sources.views.resolution = None;
    internal(project(row));
    let mut row = notification();
    row.sources.resolved.resolution.as_mut().unwrap().case_id = CaseId::new();
    internal(project(row));
    let mut row = detail();
    row.sources = notification().sources;
    internal(project(row));
    let mut row = notification();
    row.sources
        .views
        .resolution
        .as_mut()
        .unwrap()
        .reference
        .revision = FactRevision::new(2).unwrap();
    internal(project(row));
}
#[test]
fn draft_requires_exact_command_values_scope_and_result_counter() {
    let good = draft();
    let case = good.case_id;
    let command = good.command.clone();
    let value = response::draft(good.clone(), case, &command).unwrap();
    assert_eq!(value["result_revision"], 1);
    assert_eq!(value["sources_digest"], digest(2).to_hex());
    internal(response::draft(good.clone(), CaseId::new(), &command));
    internal(response::draft(good.clone(), case, &draft().command));
    let mut bad = good.clone();
    bad.result_revision = FactRevision::new(2).unwrap();
    internal(response::draft(bad, case, &command));
    let mut bad = good;
    bad.values = notification().snapshot.values();
    internal(response::draft(bad, case, &command));
}
#[test]
fn partial_local_time_does_not_gain_an_offset_or_seconds() {
    let mut row = detail();
    let ProceduralFactSnapshot::Resolution(snapshot) = &mut row.snapshot else {
        unreachable!()
    };
    let original = snapshot.values.clone();
    snapshot.values = ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: None,
        issuer: original.issuer().clone(),
        issued_at: DeclaredProceduralTime::minute("2026-09-16".parse().unwrap(), 12, 34, None)
            .unwrap(),
        summary: original.summary().clone(),
        provenance: original.provenance().clone(),
    });
    let value = project(row).unwrap();
    assert_eq!(
        value["values"]["issued_at"],
        json!({"precision":"minute","year":2026,"month":9,"day":16,"hour":12,"minute":34,"offset_seconds":null})
    );
}
#[test]
fn list_rejects_foreign_scope_duplicates_and_false_cursor() {
    let row = detail();
    let case = row.snapshot.case_id();
    let overview = resolution_overview(&row);
    let valid = ResolutionPage {
        resolutions: vec![overview.clone()],
        has_more: false,
        next_after_id: None,
    };
    assert_eq!(
        response::resolutions(valid.clone(), case).unwrap()["resolutions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    internal(response::resolutions(valid.clone(), CaseId::new()));
    let mut bad = valid.clone();
    bad.resolutions.push(overview);
    internal(response::resolutions(bad, case));
    let mut bad = valid;
    bad.has_more = true;
    internal(response::resolutions(bad, case));
}
#[test]
fn notification_list_requires_immutable_parent_and_matching_selected_parent() {
    let row = notification();
    let case = row.snapshot.case_id();
    let overview = notification_overview(&row);
    let parent = overview.root.resolution_id();
    let valid = NotificationPage {
        notifications: vec![overview],
        has_more: false,
        next_after_id: None,
    };
    assert_eq!(
        response::notifications(valid.clone(), case, parent).unwrap()["notifications"][0]
            ["resolution_id"],
        parent.to_string()
    );
    internal(response::notifications(
        valid.clone(),
        case,
        ResolutionId::new(),
    ));
    let mut bad = valid;
    bad.notifications[0].resolution.id = ResolutionId::new();
    internal(response::notifications(bad, case, parent));
}
#[test]
fn history_is_descending_exact_scoped_and_receipt_checked() {
    let row = detail();
    let case = row.snapshot.case_id();
    let target = row.snapshot.target();
    let entry = FactHistoryEntry::from(&row.snapshot);
    let valid = FactHistoryPage {
        revisions: vec![entry.clone()],
        has_more: false,
        next_before_revision: None,
    };
    assert_eq!(
        response::history(valid.clone(), case, target).unwrap()["revisions"][0]["revision"],
        1
    );
    internal(response::history(valid.clone(), CaseId::new(), target));
    let mut bad = valid.clone();
    bad.revisions.push(entry);
    internal(response::history(bad, case, target));
    let mut bad = valid.clone();
    bad.next_before_revision = Some(FactRevision::initial());
    internal(response::history(bad, case, target));
    let mut bad = valid;
    bad.revisions[0].metadata.receipt.expected_revision = 7;
    internal(response::history(bad, case, target));
}

#[path = "source_response_tests.rs"]
mod source_response_tests;
