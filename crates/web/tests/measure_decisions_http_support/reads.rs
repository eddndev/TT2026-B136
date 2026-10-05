use super::*;
use domain::hearings::HearingNote;

fn detail_path(row: &MeasureDecisionRecordReceipt) -> String {
    format!("{}/{}", base(), row.origin().decision_id)
}
fn operation_path(row: &MeasureDecisionRecordReceipt) -> String {
    format!("{}/operations/{}", base(), row.origin().operation_id)
}
fn origin_json(row: &MeasureDecisionRecordReceipt) -> Value {
    let v = row.origin();
    json!({"case_id":v.case_id.to_string(),"operation_id":v.operation_id.to_string(),
        "decision_id":v.decision_id.to_string(),"submission_digest":v.submission_digest.to_hex(),
        "review_digest":v.review_digest.to_hex(),"decision_digest":v.decision_digest.to_hex(),
        "group_digest":v.group_digest.to_hex()})
}
fn no_change() -> MeasureDecisionRecordReceipt {
    let MeasureDecisionRecordReceipt::V1(original) = g1() else {
        panic!("expected G1 fixture")
    };
    let review = &original.group.review;
    let mut cmd = review.command.clone();
    cmd.operation_id = MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(9800));
    cmd.decision_id = MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(9801));
    cmd.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(
        HearingNote::new("No measure change was declared").unwrap(),
    ))
    .unwrap();
    let history = MeasureDecisionRecordHistoryEvidence {
        records: application::measure_corrections::MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence { groups: vec![] },
            administrative: vec![],
        },
        decisions: vec![],
    };
    let group = prepare_measure_decision_with_record_history(
        &Hasher,
        &review.actor,
        review.case_id,
        cmd,
        MeasureDecisionMaterialV2 {
            context: review.material.context.clone(),
            support: review.material.support.clone(),
            anchor: None,
            predecessors: vec![],
            result_sources: vec![],
        },
        &history,
    )
    .unwrap()
    .into_group_capture(&Hasher, original.group.recorded_at)
    .unwrap();
    let origin = measure_group_origin_v2(&Hasher, &group, &history).unwrap();
    MeasureDecisionRecordReceipt::V2(Box::new(MeasureDecisionRecordStoredOperation {
        group,
        origin,
        record_history: history,
    }))
}

#[tokio::test]
async fn detail_and_operation_reads_preserve_original_g1_and_mixed_g2_receipts() {
    for row in [g1(), g2()] {
        for operation in [false, true] {
            let mut read = MockRead::new();
            let returned = row.clone();
            let path = if operation {
                let id = row.origin().operation_id;
                read.expect_get_operation()
                    .times(1)
                    .return_once(move |token, case, o| {
                        assert_eq!((token, case, o), ("staff-token", case_id(), id));
                        Ok(returned)
                    });
                operation_path(&row)
            } else {
                let id = row.origin().decision_id;
                read.expect_get()
                    .times(1)
                    .return_once(move |token, case, d| {
                        assert_eq!((token, case, d), ("staff-token", case_id(), id));
                        Ok(returned)
                    });
                detail_path(&row)
            };
            let (status, body) = request(MockWrite::new(), read, "GET", &path, None).await;
            assert_eq!(status, 200, "{body}");
            assert_eq!(body["origin"], origin_json(&row));
            assert_eq!(
                body["group"]["capture_digest"],
                row.origin().group_digest.to_hex()
            );
            assert_eq!(
                body["group"]["decision"]["capture_digest"],
                row.origin().decision_digest.to_hex()
            );
            assert_eq!(body["group"]["review"]["command"], command(row.command()));
            match row {
                MeasureDecisionRecordReceipt::V1(_) => {
                    assert_eq!(body["family"], "g1");
                    assert_eq!(body["group"]["family"], "g1");
                    assert_eq!(body["group"]["measures"][0]["family"], "m1");
                    assert_eq!(body["measure_history"], json!({"groups":[]}));
                    assert!(body.get("record_history").is_none());
                }
                MeasureDecisionRecordReceipt::V2(ref v) => {
                    assert_eq!(body["family"], "g2");
                    assert_eq!(body["group"]["family"], "g2");
                    assert_eq!(body["group"]["measures"][0]["family"], "m2");
                    assert_eq!(
                        body["group"]["review"]["material"]["predecessors"][0]["family"],
                        "c1"
                    );
                    assert_eq!(
                        body["record_history"]["records"]["judicial"]["groups"]
                            .as_array()
                            .unwrap()
                            .len(),
                        v.record_history.records.judicial.groups.len()
                    );
                    assert_eq!(
                        body["record_history"]["records"]["administrative"]
                            .as_array()
                            .unwrap()
                            .len(),
                        v.record_history.records.administrative.len()
                    );
                    assert!(body.get("measure_history").is_none());
                }
            }
        }
    }
}

fn page(items: Vec<MeasureDecisionRecordReceipt>) -> MeasureDecisionRecordPage {
    MeasureDecisionRecordPage {
        case_id: case_id(),
        items,
        has_more: false,
        next_after_id: None,
    }
}
async fn listed(
    query: MeasureDecisionReadQuery,
    returned: MeasureDecisionRecordPage,
) -> (u16, Value) {
    let mut read = MockRead::new();
    read.expect_list()
        .times(1)
        .return_once(move |token, case, q| {
            assert_eq!((token, case, q), ("staff-token", case_id(), query));
            Ok(returned)
        });
    let mut path = format!("{}?limit={}", base(), query.limit());
    if let Some(id) = query.after_id() {
        path.push_str(&format!("&after_id={id}"));
    }
    request(MockWrite::new(), read, "GET", &path, None).await
}

#[tokio::test]
async fn immutable_decision_list_includes_no_measure_change_and_exact_continuation() {
    let mut items = vec![g1(), no_change()];
    items.sort_by_key(|row| row.origin().decision_id.as_uuid());
    let last = items.last().unwrap().origin().decision_id;
    let mut returned = page(items);
    returned.has_more = true;
    returned.next_after_id = Some(last);
    let cursor = MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(1));
    let query = MeasureDecisionReadQuery::new(2, Some(cursor)).unwrap();
    let (status, body) = listed(query, returned).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["has_more"], true);
    assert_eq!(body["next_after_id"], last.to_string());
    let no_change = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["group"]["review"]["command"]["outcome"]["kind"] == "no_measure_change")
        .unwrap();
    assert_eq!(no_change["group"]["measures"], json!([]));
    assert_eq!(no_change["group"]["review"]["results"], json!([]));
}

#[tokio::test]
async fn default_decision_list_returns_empty_without_a_continuation() {
    let mut read = MockRead::new();
    read.expect_list().times(1).return_once(|token, case, q| {
        assert_eq!(
            (token, case, q),
            (
                "staff-token",
                case_id(),
                MeasureDecisionReadQuery::default()
            )
        );
        Ok(page(vec![]))
    });
    let (status, body) = request(MockWrite::new(), read, "GET", &base(), None).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body,
        json!({"case_id":case_id().to_string(),"items":[],"has_more":false,"next_after_id":null})
    );
}

#[tokio::test]
async fn decision_pages_reject_scope_order_duplicates_size_and_continuation_mismatches() {
    let mut rows = vec![g1(), g2()];
    rows.sort_by_key(|row| row.origin().decision_id.as_uuid());
    let first = rows[0].origin().decision_id;
    let valid = page(rows);
    let mut cases = Vec::new();
    let mut wrong = valid.clone();
    wrong.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(999));
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.items.reverse();
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.items[1] = wrong.items[0].clone();
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.items.push(no_change());
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.has_more = true;
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.has_more = true;
    wrong.next_after_id = Some(first);
    cases.push(wrong);
    let mut wrong = valid.clone();
    wrong.next_after_id = Some(first);
    cases.push(wrong);
    let mut wrong = page(vec![valid.items[0].clone()]);
    wrong.has_more = true;
    wrong.next_after_id = Some(first);
    cases.push(wrong);
    for returned in cases {
        let (status, body) =
            listed(MeasureDecisionReadQuery::new(2, None).unwrap(), returned).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body, internal());
    }
    let (status, body) = listed(
        MeasureDecisionReadQuery::new(2, Some(first)).unwrap(),
        valid,
    )
    .await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body, internal());
}

#[tokio::test]
async fn decision_reads_preserve_auth_absence_budget_and_generic_integrity_errors() {
    let id = g1().origin().decision_id;
    for (error, status) in [
        (ApplicationError::InvalidSession, 401),
        (ApplicationError::PermissionDenied, 403),
        (
            ApplicationError::MeasureDecision(MeasureDecisionError::NotFound),
            404,
        ),
        (
            ApplicationError::MeasureDecision(MeasureDecisionError::IncompleteHistory),
            422,
        ),
        (
            ApplicationError::MeasureDecision(MeasureDecisionError::StoredInconsistent(
                "private source metadata".into(),
            )),
            500,
        ),
    ] {
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |_, _, _| Err(error));
        let (actual, body) = request(
            MockWrite::new(),
            read,
            "GET",
            &format!("{}/{id}", base()),
            None,
        )
        .await;
        assert_eq!(actual, status, "{body}");
        if status == 500 {
            assert_eq!(body, internal());
        }
    }
}

#[tokio::test]
async fn decision_read_queries_reject_unknown_duplicate_and_noncanonical_selectors() {
    let row = g1();
    let id = row.origin().decision_id;
    for query in [
        "limit=0".to_string(),
        "limit=21".into(),
        "limit=01".into(),
        "limit=1&limit=2".into(),
        "limit=-1".into(),
        "limit=1e1".into(),
        "after_id=not-a-uuid".into(),
        format!("after_id={id}&after_id={id}"),
        "active=true".into(),
    ] {
        let (status, body) = request(
            MockWrite::new(),
            MockRead::new(),
            "GET",
            &format!("{}?{query}", base()),
            None,
        )
        .await;
        assert_eq!(status, 400, "{query}: {body}");
    }
    for path in [
        format!("{}?limit=1", detail_path(&row)),
        format!(
            "{}?capture_digest={}",
            operation_path(&row),
            row.origin().group_digest.to_hex()
        ),
    ] {
        let (status, body) = request(MockWrite::new(), MockRead::new(), "GET", &path, None).await;
        assert_eq!(status, 400, "{body}");
    }
}

#[tokio::test]
async fn decision_read_without_bearer_never_invokes_a_port() {
    let (status, body) = raw(
        (MockWrite::new(), MockRead::new()),
        "GET",
        &base(),
        "",
        String::new(),
        "application/json",
    )
    .await;
    assert_eq!(status, 401, "{body}");
}
