use super::*;

#[tokio::test]
async fn resource_hearing_cursor_uses_rank_two_and_binds_range_kind_and_status() {
    let mut value = own_page();
    value.complete = false;
    value.next_after = Some(value.items[0].key().unwrap());
    let (_, router) = setup(value.clone());
    let query = format!("{RANGE}&kind=resource_hearing&hearing_status=all&limit=1");
    let (status, body) = request(router, &query, true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let cursor = body["next_cursor"].as_str().unwrap();
    assert_eq!(
        cursor,
        format!(
            "a1:1767225600:1767312000:resource_hearing:all:1767225601:0:2:{}",
            Uuid::from_u128(2)
        )
    );
    let (workflow, router) = setup(empty_page());
    let query = format!("{RANGE}&kind=resource_hearing&hearing_status=all&limit=2&cursor={cursor}");
    assert_eq!(request(router, &query, true).await.0, StatusCode::OK);
    assert_eq!(
        workflow.calls.lock().unwrap()[0].1.after(),
        value.next_after
    );
    for query in [
        format!("{RANGE}&kind=all&hearing_status=all&cursor={cursor}"),
        format!("{RANGE}&kind=resource_hearing&hearing_status=scheduled&cursor={cursor}"),
        format!("from=2026-01-01T00:00:00Z&until=2026-01-03T00:00:00Z&kind=resource_hearing&hearing_status=all&cursor={cursor}"),
        format!("{RANGE}&kind=resource_hearing&hearing_status=all&cursor={}", cursor.replace(":0:2:", ":0:3:")),
        format!("{RANGE}&kind=resource_hearing&hearing_status=all&cursor={}", cursor.replace(":0:2:", ":0:0:")),
    ] {
        let (workflow, router) = setup(empty_page());
        let (status, body) = request(router, &query, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn rank_two_partial_empty_pages_preserve_nanos_and_existing_family_ranks() {
    for (kind, rank) in [
        (AgendaItemKind::Hearing, 0),
        (AgendaItemKind::Deadline, 1),
        (AgendaItemKind::ResourceHearing, 2),
    ] {
        let mut value = empty_page();
        value.complete = false;
        value.next_after = Some(
            AgendaCursor::new(
                at(1767225601).replace_nanosecond(999999999).unwrap(),
                kind,
                Uuid::from_u128(2),
            )
            .unwrap(),
        );
        let (_, router) = setup(value.clone());
        let (status, body) = request(router, RANGE, true).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["items"], json!([]));
        let cursor = body["next_cursor"].as_str().unwrap();
        assert_eq!(
            cursor,
            format!(
                "a1:1767225600:1767312000:all:scheduled:1767225601:999999999:{rank}:{}",
                Uuid::from_u128(2)
            )
        );
        let (workflow, router) = setup(empty_page());
        assert_eq!(
            request(router, &format!("{RANGE}&cursor={cursor}"), true)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            workflow.calls.lock().unwrap()[0].1.after(),
            value.next_after
        );
    }
}
