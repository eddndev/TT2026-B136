use super::*;
use domain::audit::{AuditContext, AuditSource};
use std::net::Ipv4Addr;
use uuid::Uuid;

fn context() -> AuditContext {
    AuditContext {
        user_id: Some(UserId::from_uuid(Uuid::from_u128(0xabcdef))),
        ip_address: Some(Ipv4Addr::new(192, 0, 2, 1).into()),
        source: AuditSource::Http,
        request_id: Some(Uuid::from_u128(2)),
    }
}

fn record() -> AuditEvent {
    let mut record = event(0, 0);
    record.actor = "historical-actor".into();
    record.context = Some(context());
    record
}

fn actor_query(filter: &str, cursor: Option<&str>) -> AuditEventQuery {
    AuditEventQuery::new(
        at(),
        at() + Duration::days(1),
        Some(filter),
        None,
        None,
        1,
        cursor,
    )
    .unwrap()
}

#[test]
fn actor_filter_matches_historical_text_or_the_exact_recorded_user_uuid() {
    let record = record();
    let user = context().user_id.unwrap().to_string();
    for filter in [record.actor.as_str(), user.as_str()] {
        let owner = actor(Role::Owner);
        let (service, trace, _) = workflow(
            vec![Ok(owner.clone()), Ok(owner)],
            Ok(batch(vec![record.clone()], false)),
        );
        let page = service.read("session", actor_query(filter, None)).unwrap();
        assert_eq!(page.events.as_slice(), std::slice::from_ref(&record));
        assert_eq!(*trace.lock().unwrap(), ["auth", "store", "auth"]);
    }
}

#[test]
fn actor_uuid_does_not_resolve_email_or_accept_other_uuid_spellings() {
    let mut owner = actor(Role::Owner);
    let user = context().user_id.unwrap();
    owner.id = user;
    for filter in [
        owner.email.clone(),
        user.to_string().to_uppercase(),
        user.as_uuid().simple().to_string(),
        user.as_uuid().urn().to_string(),
        Uuid::from_u128(3).to_string(),
    ] {
        let (service, trace, _) = workflow(
            vec![Ok(owner.clone()), Ok(owner.clone())],
            Ok(batch(vec![record()], false)),
        );
        assert!(matches!(
            service.read("session", actor_query(&filter, None)),
            Err(ApplicationError::Port(_))
        ));
        assert_eq!(*trace.lock().unwrap(), ["auth", "store"]);
    }
}

#[test]
fn uuid_filter_preserves_snapshot_continuation_and_exact_filter_binding() {
    let user = context().user_id.unwrap().to_string();
    let q = actor_query(&user, None);
    let owner = actor(Role::Owner);
    let first = record();
    let (service, _, _) = workflow(
        vec![Ok(owner.clone()), Ok(owner.clone())],
        Ok(batch(vec![first.clone()], true)),
    );
    let page = service.read("session", q.clone()).unwrap();
    let cursor = page.next_cursor.unwrap();
    assert_eq!(cursor, q.cursor_after(20, &first).unwrap());
    assert!(AuditEventQuery::new(
        at(),
        at() + Duration::days(1),
        Some(&first.actor),
        None,
        None,
        1,
        Some(&cursor),
    )
    .is_err());
    let next_query = actor_query(&user, Some(&cursor));
    assert_eq!(next_query.cursor().unwrap().snapshot_max_sequence, 20);
    let mut next = first;
    next.sequence = 1;
    let (service, _, _) = workflow(
        vec![Ok(owner.clone()), Ok(owner)],
        Ok(batch(vec![next.clone()], false)),
    );
    let page = service.read("session", next_query).unwrap();
    assert_eq!(page.events, [next]);
    assert_eq!(page.next_cursor, None);
}

#[test]
fn metadata_json_counts_toward_the_exact_page_capacity() {
    for context in [
        context(),
        AuditContext {
            user_id: None,
            ip_address: None,
            source: AuditSource::Internal,
            request_id: None,
        },
    ] {
        let metadata_bytes = serde_json::to_vec(&context).unwrap().len();
        for extra in [0, 1] {
            let mut record = event(0, 0);
            record.context = Some(context.clone());
            record.resource = "r".repeat(
                MAX_AUDIT_PAGE_TEXT_BYTES
                    - record.actor.len()
                    - record.action.len()
                    - metadata_bytes
                    + extra,
            );
            let owner = actor(Role::Owner);
            let (service, trace, _) = workflow(
                vec![Ok(owner.clone()), Ok(owner)],
                Ok(batch(vec![record.clone()], false)),
            );
            let result = service.read("session", query(20, None));
            if extra == 0 {
                assert_eq!(result.unwrap().events, [record]);
                assert_eq!(*trace.lock().unwrap(), ["auth", "store", "auth"]);
            } else {
                assert!(matches!(
                    result,
                    Err(ApplicationError::AuditQueryCapacityExceeded)
                ));
                assert_eq!(*trace.lock().unwrap(), ["auth", "store"]);
            }
        }
    }
}

#[test]
fn metadata_capacity_is_aggregated_across_the_whole_page() {
    let first = record();
    let mut second = event(1, 0);
    second.context = Some(context());
    let metadata_bytes = serde_json::to_vec(&context()).unwrap().len();
    second.resource = "r".repeat(
        MAX_AUDIT_PAGE_TEXT_BYTES
            - first.actor.len()
            - first.action.len()
            - first.resource.len()
            - second.actor.len()
            - second.action.len()
            - 2 * metadata_bytes
            + 1,
    );
    let owner = actor(Role::Owner);
    let (service, trace, _) = workflow(
        vec![Ok(owner.clone()), Ok(owner)],
        Ok(batch(vec![first, second], false)),
    );
    assert!(matches!(
        service.read("session", query(20, None)),
        Err(ApplicationError::AuditQueryCapacityExceeded)
    ));
    assert_eq!(*trace.lock().unwrap(), ["auth", "store"]);
}
