use super::*;
use domain::precautionary_hearings::PrecautionaryHearingId;

fn caution() -> AlertRecord {
    let mut value = captured();
    value.subject = AlertSubject::PrecautionaryHearing {
        case_id: value.subject.case_id(),
        id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(5)),
    };
    value.subject_title = "Captured precautionary hearing".into();
    value.origin.revision = 2;
    value
}

fn expected_subject() -> Value {
    json!({"kind":"precautionary_hearing", "case_id":Uuid::from_u128(4).to_string(),
        "id":Uuid::from_u128(5).to_string()})
}

#[tokio::test]
async fn precautionary_inbox_and_detail_keep_revision_two_without_resource_parent() {
    let alert = caution();
    let app = router(alert.clone());
    let (status, page) = request(app.clone(), "GET", "/api/v1/alerts", None, true).await;
    assert_eq!(status, StatusCode::OK);
    let (status, detail) =
        request(app, "GET", &format!("/api/v1/alerts/{}", id()), None, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["alerts"][0], detail["alert"]);
    let value = &detail["alert"];
    assert_eq!(value["subject"], expected_subject());
    assert_eq!(
        value["origin"],
        json!({"revision":2,
        "evidence_digest":alert.origin.evidence_digest.to_hex()})
    );
    assert_eq!(value["kind"]["kind"], "upcoming");
    assert_eq!(value["kind"]["lead_hours"], 24);
    assert_eq!(
        value["kind"]["activity_at"]["unix_seconds"],
        (at() + Duration::hours(24)).unix_timestamp()
    );
    assert_eq!(value["kind"]["activity_at"]["nanosecond"], 123456789);
    assert_eq!(value["trigger_at"]["nanosecond"], 123456789);
}

#[tokio::test]
async fn precautionary_read_receipt_retains_historical_origin_after_cancellation() {
    let mut alert = caution();
    alert.state = AlertState::Resolved {
        at: at(),
        reason: AlertResolutionReason::CancelledHearing,
    };
    let app = router(alert.clone());
    let command = json!({"operation_id":Uuid::from_u128(9).to_string()});
    let mut prior = None;
    for _ in 0..2 {
        let (status, body) = request(
            app.clone(),
            "POST",
            &format!("/api/v1/alerts/{}/read", id()),
            Some(&command.to_string()),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["operation_id"], command["operation_id"]);
        assert_eq!(body["alert"]["subject"], expected_subject());
        assert_eq!(
            body["alert"]["origin"],
            json!({"revision":2,
            "evidence_digest":alert.origin.evidence_digest.to_hex()})
        );
        assert_eq!(body["alert"]["state"]["reason"], "cancelled_hearing");
        assert_ne!(body["alert"]["read_at"], Value::Null);
        if let Some(expected) = prior {
            assert_eq!(body, expected);
        }
        prior = Some(body);
    }
}

#[tokio::test]
async fn precautionary_zero_revision_or_non_upcoming_kind_fails_without_disclosure() {
    let mut zero = caution();
    zero.origin.revision = 0;
    let mut invalid = vec![zero];
    for kind in [
        AlertKind::ReviewRequired,
        AlertKind::OverdueUnattended { due_at: at() },
        AlertKind::DueChangedSoon {
            previous_due_at: at() + Duration::hours(48),
            current_due_at: at() + Duration::hours(24),
        },
    ] {
        let mut value = caution();
        value.kind = kind;
        invalid.push(value);
    }
    for value in invalid {
        let app = router(value);
        for path in [
            "/api/v1/alerts".to_owned(),
            format!("/api/v1/alerts/{}", id()),
        ] {
            let (status, body) = request(app.clone(), "GET", &path, None, true).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(
                body,
                json!({"error":{"code":"internal_error",
                "message":"internal application error"}})
            );
        }
    }
}
