mod resource_hearing_activity_support;
use application::resource_activities::*;
use resource_hearing_activity_support::*;
use serde_json::json;

#[tokio::test]
async fn resource_hearing_selection_round_trips_as_its_own_typed_family() {
    let mut workflow = MockWorkflow::new();
    workflow
        .expect_prepare()
        .times(1)
        .return_once(|_, c, r, actual| {
            assert_eq!((c, r), (case(), resource()));
            assert_eq!(actual, command());
            Ok(draft())
        });
    let input = body();
    let (status, value) = request(
        workflow,
        "POST",
        &format!("{}/prepare", base()),
        Some(input.clone()),
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["command"], input);
    assert_eq!(value["selection"]["target"]["kind"], "resource_hearing");
    assert_eq!(value["sources"]["target"]["kind"], "resource_hearing");
    let record = &value["sources"]["target"]["record"];
    assert_eq!(record["id"], hearing_id().to_string());
    assert_eq!(record["resource"]["revision"], 2);
    assert_eq!(record["recorded_resource_head"]["revision"], 5);
    assert_eq!(record["values"]["kind"], "written_revocation");
    assert_eq!(record["values"]["scheduled_at"], "1970-01-03T00:00:00Z");
    assert_eq!(
        record["values"]["scheduling_basis"]["support"]["version"],
        3
    );
    assert_eq!(record["sources"]["support"]["digest"], "2a".repeat(32));
    assert_eq!(record["sources"]["resource"]["revision"], 2);
    assert!(record.get("scheduling_context").is_none());
    assert!(record.get("recorded_stage").is_none());
}

#[tokio::test]
async fn exact_association_projects_capture_and_current_resource_hearing_separately() {
    let mut workflow = MockWorkflow::new();
    workflow
        .expect_get()
        .times(1)
        .return_once(|_, _, _, _, revision| {
            assert_eq!(revision, Some(ResourceActivityRevision::initial()));
            Ok(view())
        });
    let (status, value) = request(
        workflow,
        "GET",
        &format!("{}/{}/revisions/1", base(), id()),
        None,
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["current_target"]["kind"], "resource_hearing");
    let captured = &value["association"]["sources"]["target"]["record"];
    assert_eq!(captured, &value["current_target"]["record"]);
    assert_eq!(captured["capture_digest"], digest().to_hex());
    assert_eq!(captured["case_id"], case().to_string());
    assert_eq!(captured["resource_id"], resource().to_string());
    assert_eq!(value["checked_at"]["unix_seconds"], 0);
}

#[tokio::test]
async fn list_filter_preserves_the_resource_hearing_family() {
    let mut workflow = MockWorkflow::new();
    workflow
        .expect_list()
        .times(1)
        .return_once(|_, _, _, query| {
            assert_eq!(query.kind(), Some(ResourceActivityKind::ResourceHearing));
            Ok(ResourceActivityPage {
                associations: vec![view()],
                has_more: false,
                next_after_id: None,
            })
        });
    let (status, value) = request(
        workflow,
        "GET",
        &format!("{}?kind=resource_hearing", base()),
        None,
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(
        value["associations"][0]["current_target"]["kind"],
        "resource_hearing"
    );
}

#[tokio::test]
async fn target_parser_rejects_ordinary_receipt_fields_and_invalid_exact_references() {
    for fault in 0..5 {
        let mut input = body();
        let target = &mut input["change"]["target"];
        match fault {
            0 => {
                target["submission_digest"] = json!(digest().to_hex());
            }
            1 => {
                target.as_object_mut().unwrap().remove("capture_digest");
            }
            2 => target["revision"] = json!(0),
            3 => target["capture_digest"] = json!("AB".repeat(32)),
            _ => target["kind"] = json!("hearing"),
        }
        let (status, value) = request(
            MockWorkflow::new(),
            "POST",
            &format!("{}/prepare", base()),
            Some(input),
        )
        .await;
        assert_eq!(status, 400, "fault {fault}: {value}");
    }
}

#[tokio::test]
async fn inconsistent_resource_hearing_context_or_capture_never_escapes_projection() {
    for fault in 0..6 {
        let mut returned = view();
        if fault < 4 {
            let ResourceActivityTargetDetail::ResourceHearing(h) =
                &mut returned.association.sources.target
            else {
                unreachable!()
            };
            match fault {
                0 => h.review.command.resource.id = ResourceId::new(),
                1 => h.capture_digest = domain::crypto::Sha256Digest::from_array([9; 32]),
                2 => h.review.support.digest = domain::crypto::Sha256Digest::from_array([9; 32]),
                _ => {
                    h.review.command.hearing_id =
                        domain::resource_hearings::ResourceHearingId::new()
                }
            }
        } else {
            let ResourceActivityCurrentTarget::ResourceHearing(h) = &mut returned.current_target
            else {
                unreachable!()
            };
            if fault == 4 {
                h.review.command.hearing_id = domain::resource_hearings::ResourceHearingId::new();
            } else {
                h.recorded_at += time::Duration::seconds(1);
            }
        }
        let mut workflow = MockWorkflow::new();
        workflow
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(returned));
        let (status, value) = request(workflow, "GET", &format!("{}/{}", base(), id()), None).await;
        assert_eq!(status, 500, "fault {fault}: {value}");
        assert_eq!(
            value,
            json!({"error":{"code":"internal_error","message":"internal application error"}})
        );
    }
}
