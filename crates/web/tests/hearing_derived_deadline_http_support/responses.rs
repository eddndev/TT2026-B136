use super::*;

#[tokio::test]
async fn ready_contains_reviewed_material_and_blocked_result_without_fabricated_capture() {
    let (draft, _) = records::fixture();
    let workflow = Arc::new(Workflow {
        ready: Some(draft),
        ..Workflow::default()
    });
    let (status, body) = prepare(workflow, command()).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["state"], "ready");
    assert_eq!(body["command"]["case_id"], CASE);
    assert_eq!(body["command"]["result"]["result_id"], RESULT);
    assert_eq!(body["result"]["result_revision"], 1);
    assert_eq!(body["result"]["values"]["event_time"]["precision"], "date");
    assert_eq!(
        body["deadline"]["definition"]["input"]["selection"]["source"]["value"]["result_id"],
        RESULT
    );
    assert_eq!(
        body["deadline"]["tracking"],
        deadline_http_support::tracking()
    );
    assert_eq!(body["deadline"]["profile"]["revision"], 1);
    assert_eq!(body["deadline"]["profile_head"]["revision"], 1);
    assert_eq!(
        body["deadline"]["responsible"]["id"],
        deadline_http_support::ACTOR
    );
    assert!(body["deadline"]["calendar"].is_null());
    assert!(body["deadline"]["calendar_head"].is_null());
    assert!(body["deadline"]["result"]["due_at"].is_null());
    assert!(!body["deadline"]["result"]["blocks"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        body["review_digest"],
        deadline_http_support::digest().to_hex()
    );
    for target in [&body, &body["result"], &body["deadline"]] {
        for key in ["recorded_at", "source_event", "capture_digest", "receipt"] {
            assert!(target.get(key).is_none(), "unexpected {key}: {target}");
        }
    }
    assert!(body["deadline"].get("source").is_none());
}

#[tokio::test]
async fn replay_and_submit_preserve_the_original_pair_origin_and_large_event_identity() {
    let (_, record) = records::fixture();
    let workflow = Arc::new(Workflow {
        replay: Some(record),
        ..Workflow::default()
    });
    let (status, first) = prepare(workflow.clone(), command()).await;
    assert_eq!(status, 200, "{first}");
    assert_eq!(first["state"], "replay");
    let (status, second) = request(
        workflow.clone(),
        "/submit",
        Some("owner"),
        submission().to_string(),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 201, "{second}");
    assert_eq!(first["record"], second);
    assert_eq!(second["case_id"], CASE);
    assert_eq!(second["result"]["id"], RESULT);
    assert_eq!(second["result"]["revision"], 1);
    assert_eq!(second["deadline"]["id"], deadline_http_support::ID);
    assert_eq!(second["deadline"]["revision"], 1);
    assert_eq!(
        second["origin"]["source_event"]["sequence"],
        records::EVENT_SEQUENCE.to_string()
    );
    assert_eq!(second["origin"]["source_event"]["revision"], 1);
    assert_eq!(second["origin"]["review_digest"], second["review_digest"]);
    assert_eq!(second["origin"]["capture_digest"], second["capture_digest"]);
    assert!(!second["result"]["recorded_at"].is_null());
    assert!(!second["deadline"]["recorded_at"].is_null());
    let (status, again) = prepare(workflow.clone(), command()).await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(first, again);
    assert_eq!(workflow.calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn contradictory_ready_replay_and_submit_commands_fail_closed() {
    let (draft, record) = records::fixture();
    let mut changed = command();
    changed["deadline"]["change"]["definition"]["title"] = json!("Different reviewed instruction");
    for replay in [false, true] {
        let workflow = Arc::new(Workflow {
            ready: (!replay).then(|| draft.clone()),
            replay: replay.then(|| record.clone()),
            ..Workflow::default()
        });
        let (status, body) = prepare(workflow, changed.clone()).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body["error"]["code"], "internal_error");
    }
    let workflow = Arc::new(Workflow {
        replay: Some(record),
        ..Workflow::default()
    });
    let mut sent = submission();
    sent["expected_review_digest"] = json!("11".repeat(32));
    let (status, body) = request(
        workflow,
        "/submit",
        Some("owner"),
        sent.to_string(),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body["error"]["code"], "internal_error");
}

#[tokio::test]
async fn equal_instants_with_different_declared_offsets_are_not_equal_http_commands() {
    use application::hearing_results::DeclaredHearingResultTime;
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};
    let at = OffsetDateTime::parse("2026-09-16T09:00:00-06:00", &Rfc3339).unwrap();
    let (draft, record) =
        records::fixture_with_time(DeclaredHearingResultTime::instant(at).unwrap());
    let mut input = command();
    input["result"]["change"]["values"]["event_time"] =
        json!({"precision":"instant","at":"2026-09-16T15:00:00Z"});
    for replay in [false, true] {
        let workflow = Arc::new(Workflow {
            ready: (!replay).then(|| draft.clone()),
            replay: replay.then(|| record.clone()),
            ..Workflow::default()
        });
        let (status, body) = prepare(workflow, input.clone()).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body["error"]["code"], "internal_error");
    }
}
