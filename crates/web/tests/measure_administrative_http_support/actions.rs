use super::*;

#[tokio::test]
async fn mark_is_one_retained_record_without_replacement_or_invented_judicial_action() {
    let operation = marked();
    let review = operation.capture.review.clone();
    let mut write = MockWrite::new();
    let expected = operation.clone();
    write
        .expect_submit()
        .times(1)
        .return_once(move |_, case, command, confirmation| {
            assert_eq!(case, expected.origin.case_id);
            assert_eq!(command, expected.capture.review.command);
            assert_eq!(confirmation, confirm(&expected));
            Ok(expected)
        });
    let (status, body) = request(
        write,
        MockRead::new(),
        "POST",
        &submit_path(),
        Some(submit_json(&operation)),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    assert_stored(&body, &operation);
    assert_eq!(
        body["capture"]["review"]["command"]["action"],
        json!({"kind":"entered_in_error"})
    );
    assert_eq!(
        body["capture"]["records"][0]["result"]["validity"],
        "entered_in_error"
    );
    assert_eq!(
        body["capture"]["records"][0]["result"]["last_action"],
        "impose"
    );
    assert_eq!(
        body["capture"]["records"][0]["result"]["previous"],
        reference(review.command.target)
    );
    assert!(body["capture"]["replacement_link"].is_null());
}

#[tokio::test]
async fn replacement_keeps_both_ids_and_explicit_roles_even_when_new_uuid_sorts_first() {
    let operation = replacement();
    let original = &operation.capture.review.result;
    let replacement = operation.capture.review.replacement.as_ref().unwrap();
    assert!(replacement.id.as_uuid() < original.id.as_uuid());
    let mut write = MockWrite::new();
    let expected = operation.clone();
    write
        .expect_submit()
        .times(1)
        .return_once(move |_, _, command, confirmation| {
            assert_eq!(command, expected.capture.review.command);
            assert_eq!(confirmation, confirm(&expected));
            Ok(expected)
        });
    let (status, body) = request(
        write,
        MockRead::new(),
        "POST",
        &submit_path(),
        Some(submit_json(&operation)),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    assert_stored(&body, &operation);
    let records = body["capture"]["records"].as_array().unwrap();
    assert_eq!(records[0]["result"]["id"], replacement.id.to_string());
    assert_eq!(records[1]["result"]["id"], original.id.to_string());
    assert_eq!(records[0]["result"]["validity"], "valid");
    assert_eq!(records[1]["result"]["validity"], "entered_in_error");
    assert_eq!(records[0]["result"]["revision"], 1);
    assert_eq!(
        records[0]["result"]["record_root"],
        json!({"kind":"administrative",
        "operation_id":operation.origin.operation_id.to_string(),"measure_id":replacement.id.to_string()})
    );
    for record in records {
        assert_eq!(
            record["result"]["previous"],
            reference(operation.capture.review.command.target)
        );
    }
    let link = operation.capture.replacement_link.as_ref().unwrap();
    assert_eq!(
        body["capture"]["replacement_link"],
        json!({"entered_in_error":reference(link.entered_in_error),"replacement":reference(link.replacement)})
    );
    assert_eq!(
        records[0]["result"]["sources"]["subject"]["id"],
        replacement.sources.subject.id.to_string()
    );
    assert_eq!(
        records[1]["result"]["sources"]["subject"]["id"],
        original.sources.subject.id.to_string()
    );
}

#[tokio::test]
async fn replay_preserves_original_profile_nanos_sources_and_complete_mixed_owner_history() {
    let operation = after_replacement_g2();
    let review = &operation.capture.review;
    let mut write = MockWrite::new();
    let expected = operation.clone();
    write
        .expect_submit()
        .times(1)
        .return_once(move |_, _, _, _| Ok(expected));
    let (status, body) = request(
        write,
        MockRead::new(),
        "POST",
        &submit_path(),
        Some(submit_json(&operation)),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    assert_stored(&body, &operation);
    assert!(body.get("replayed").is_none());
    assert_eq!(
        body["capture"]["recorded_at"],
        utc(operation.capture.recorded_at)
    );
    assert_eq!(
        body["capture"]["review"]["actor"],
        json!({"id":review.actor.id.to_string(),"email":review.actor.email,"role":review.actor.role.as_str()})
    );
    let result = &body["capture"]["review"]["result"];
    assert_eq!(result["last_action"], "confirm");
    assert_eq!(result["record_root"]["kind"], "administrative");
    assert_eq!(
        result["last_judicial"]["reference"],
        reference(review.result.last_judicial.reference)
    );
    let subject = &review.result.sources.subject;
    assert_eq!(
        result["sources"]["subject"]["changed_at"],
        utc(subject.changed_at)
    );
    assert_eq!(
        result["sources"]["subject"]["changed_by"]["email"],
        subject.changed_by.email
    );
    let supervisor = review.result.sources.supervisor.as_ref().unwrap();
    let bound = supervisor.bound_subject.as_ref().unwrap();
    assert_eq!(result["sources"]["supervisor"]["canonical_format"], "part2");
    assert_eq!(
        result["sources"]["supervisor"]["directory_status"],
        "archived"
    );
    assert_eq!(
        result["sources"]["supervisor"]["subject"]["values_digest"],
        bound.values_digest.to_hex()
    );
    assert_eq!(
        result["sources"]["supervisor"]["subject"]["changed_at"],
        utc(bound.changed_at)
    );
    assert_eq!(
        body["capture"]["review"]["support"]["digest"],
        review.support.digest.to_hex()
    );
    let history = &body["record_history"];
    assert_eq!(
        history["records"]["judicial"]["groups"][0]["capture"]["family"],
        "g1"
    );
    assert_eq!(
        history["records"]["administrative"][0]["capture"]["records"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(history["decisions"][0]["capture"]["family"], "g2");
}

#[tokio::test]
async fn prepare_normalizes_notes_before_forwarding_and_binding_the_review() {
    let operation = correct();
    let review = operation.capture.review;
    let mut body = command_json(&review);
    body["reason"] = json!(format!("  {}  ", review.command.reason.as_str()));
    let mut write = MockWrite::new();
    let expected = review.clone();
    write
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, command| {
            assert_eq!(command, expected.command);
            Ok(expected)
        });
    let (status, response) =
        request(write, MockRead::new(), "POST", &prepare_path(), Some(body)).await;
    assert_eq!(status, 200, "{response}");
    assert_eq!(response["command"], command_json(&review));
}
