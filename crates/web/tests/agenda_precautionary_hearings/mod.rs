use super::*;
use domain::{
    crypto::Sha256Digest,
    precautionary_hearings::{
        PrecautionaryHearingId, PrecautionaryHearingPurpose, PrecautionaryHearingRevision,
    },
};
use serde_json::json;

fn own_page(status: HearingStatus) -> AgendaPage {
    AgendaPage {
        items: vec![AgendaItem::PrecautionaryHearing {
            case: AgendaCaseSummary {
                case_id: CaseId::from_uuid(Uuid::from_u128(1)),
                title: "Authorized precautionary case".into(),
                reference: "PRECAUTIONARY-1".into(),
                status: CaseAdministrativeStatus::Closed,
            },
            hearing: Box::new(PrecautionaryHearingAgendaOverview {
                case_id: CaseId::from_uuid(Uuid::from_u128(1)),
                id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(2)),
                revision: PrecautionaryHearingRevision::new(3).unwrap(),
                purpose: PrecautionaryHearingPurpose::Review,
                scheduled_at: HearingTime::new(
                    at(1767225601).to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap()),
                )
                .unwrap(),
                modality: HearingModality::InPerson,
                status,
                participant_count: 2,
                capture_digest: Sha256Digest::from_array([8; 32]),
            }),
        }],
        ..page()
    }
}
fn empty_page() -> AgendaPage {
    AgendaPage {
        items: vec![],
        ..page()
    }
}

#[tokio::test]
async fn cancelled_precautionary_projection_keeps_current_revision_exact_offset_and_capture() {
    let (workflow, router) = setup(own_page(HearingStatus::Cancelled));
    let query = format!("{RANGE}&kind=precautionary_hearing&hearing_status=cancelled");
    let (status, body) = request(router, &query, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["kind"], "precautionary_hearing");
    assert_eq!(body["hearing_status"], "cancelled");
    assert_eq!(
        body["items"],
        json!([{
            "kind":"precautionary_hearing",
            "at":{"unix_seconds":1767225601i64,"nanosecond":0,"offset_seconds":0},
            "case_title":"Authorized precautionary case",
            "case_reference":"PRECAUTIONARY-1","case_status":"closed",
            "precautionary_hearing":{
                "case_id":Uuid::from_u128(1).to_string(),
                "id":Uuid::from_u128(2).to_string(),"revision":3,
                "purpose":"review","scheduled_at":"2025-12-31T18:00:01-06:00",
                "modality":"in_person","status":"cancelled","participant_count":2,
                "capture_digest":"08".repeat(32),
            },
        }])
    );
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "agenda-session");
    assert_eq!(calls[0].1.kind(), AgendaKind::PrecautionaryHearing);
    assert_eq!(calls[0].1.hearing_status(), HearingStatusFilter::Cancelled);
}

#[tokio::test]
async fn precautionary_filter_accepts_own_statuses_and_rejects_ambiguous_queries() {
    for (status, name) in [
        (HearingStatus::Scheduled, "scheduled"),
        (HearingStatus::Cancelled, "cancelled"),
    ] {
        for kind in ["all", "precautionary_hearing"] {
            for filter in [name, "all"] {
                let (workflow, router) = setup(own_page(status));
                let query = format!("{RANGE}&kind={kind}&hearing_status={filter}");
                let (code, body) = request(router, &query, true).await;
                assert_eq!(code, StatusCode::OK, "{query}: {body}");
                assert_eq!(workflow.calls.lock().unwrap().len(), 1);
            }
        }
    }
    for suffix in [
        "&kind=precautionary_hearing&kind=hearing",
        "&kind=precautionary_hearing&hearing_status=cancelled&hearing_status=all",
        "&kind=precautionary_hearing&purpose=review",
        "&kind=precautionary_hearings",
    ] {
        let (workflow, router) = setup(empty_page());
        let (status, body) = request(router, &format!("{RANGE}{suffix}"), true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{suffix}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    for suffix in [
        "&kind=hearing",
        "&kind=resource_hearing",
        "&kind=deadline",
        "&hearing_status=cancelled",
    ] {
        let (_, router) = setup(own_page(HearingStatus::Scheduled));
        let (status, body) = request(router, &format!("{RANGE}{suffix}"), true).await;
        assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{suffix}: {body}"
        );
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn precautionary_cursor_uses_canonical_rank_three_and_retains_its_query_scope() {
    let mut value = own_page(HearingStatus::Scheduled);
    value.complete = false;
    value.next_after = Some(value.items[0].key().unwrap());
    let (_, router) = setup(value.clone());
    let query = format!("{RANGE}&kind=precautionary_hearing&hearing_status=all&limit=1");
    let (status, body) = request(router, &query, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let cursor = body["next_cursor"].as_str().unwrap();
    assert_eq!(
        cursor,
        format!(
            "a1:1767225600:1767312000:precautionary_hearing:all:1767225601:0:3:{}",
            Uuid::from_u128(2),
        )
    );
    let (workflow, router) = setup(empty_page());
    let next =
        format!("{RANGE}&kind=precautionary_hearing&hearing_status=all&limit=2&cursor={cursor}");
    assert_eq!(request(router, &next, true).await.0, StatusCode::OK);
    assert_eq!(
        workflow.calls.lock().unwrap()[0].1.after(),
        value.next_after
    );
    for query in [
        format!("{RANGE}&kind=all&hearing_status=all&cursor={cursor}"),
        format!("{RANGE}&kind=precautionary_hearing&hearing_status=scheduled&cursor={cursor}"),
        format!("from=2026-01-01T00:00:00Z&until=2026-01-03T00:00:00Z&kind=precautionary_hearing&hearing_status=all&cursor={cursor}"),
        format!("{RANGE}&kind=precautionary_hearing&hearing_status=all&cursor={}", cursor.replace(":0:3:", ":0:03:")),
        format!("{RANGE}&kind=precautionary_hearing&hearing_status=all&cursor={}", cursor.replace(":0:3:", ":0:2:")),
    ] {
        let (workflow, router) = setup(empty_page());
        let (status, body) = request(router, &query, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

fn internal() -> Value {
    json!({"error":{"code":"internal_error","message":"internal application error"}})
}
#[tokio::test]
async fn inconsistent_precautionary_case_scope_or_page_shape_is_opaque() {
    for fault in 0..4 {
        let mut value = own_page(HearingStatus::Scheduled);
        let AgendaItem::PrecautionaryHearing { case, hearing } = &mut value.items[0] else {
            unreachable!()
        };
        match fault {
            0 => case.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            1 => hearing.participant_count = 33,
            2 => hearing.scheduled_at = HearingTime::new(at(1767139200)).unwrap(),
            _ => value.items.push(value.items[0].clone()),
        }
        let (_, router) = setup(value);
        let (status, body) = request(router, RANGE, true).await;
        assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "fault {fault}: {body}"
        );
        assert_eq!(body, internal());
    }
}
