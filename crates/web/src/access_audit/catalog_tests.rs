use super::*;
use application::access_audit::{AccessAuditRecord, AccessAuditWriter};
use axum::{
    body::Body,
    http::{Method, Request},
    routing::{on, MethodFilter},
};
use std::sync::Mutex;
use tower::ServiceExt;

#[derive(Default)]
struct Sink(Mutex<Vec<AccessAuditRecord>>);
impl AccessAuditWriter for Sink {
    fn record(&self, value: AccessAuditRecord) -> Result<(), application::ApplicationError> {
        self.0.lock().unwrap().push(value);
        Ok(())
    }
}

// This exercises the common boundary, not the individual workflow permissions.
#[tokio::test]
async fn finite_route_catalog_preserves_recognized_operation_and_valid_resource_identifiers() {
    let catalog = include_str!("../../../../docs/access-audit-matrix.md");
    let mut count = 0;
    for line in catalog.lines() {
        let parts: Vec<_> = line.split('\t').collect();
        if parts.len() != 4 || !parts[1].starts_with("/api/v1/") {
            continue;
        }
        let method: Method = parts[0].parse().unwrap();
        let pattern = parts[1];
        let id = uuid::Uuid::new_v4().to_string();
        let path = pattern
            .split('/')
            .map(|part| {
                if matches!(part, ":version" | ":revision") {
                    "2"
                } else if part.starts_with(':') {
                    &id
                } else {
                    part
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        for status in [200, 401, 403, 404, 422, 500] {
            let expected = StatusCode::from_u16(status).unwrap();
            let sink = Arc::new(Sink::default());
            let router = Router::new().route(
                pattern,
                on(
                    MethodFilter::try_from(method.clone()).unwrap(),
                    move || async move { expected },
                ),
            );
            let router = with_access_audit(
                crate::runtime::protect(
                    router,
                    crate::runtime::HttpRuntime::new(crate::HttpLimits::default()),
                ),
                sink.clone(),
                AuditProxyPolicy::Direct,
            );
            let response = router
                .oneshot(
                    Request::builder()
                        .method(method.clone())
                        .uri(&path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{line}");
            let records = sink.0.lock().unwrap();
            assert_eq!(records.len(), 1, "{line}");
            assert_eq!(records[0].action, outcome(expected));
            assert!(records[0]
                .resource
                .starts_with(&format!("{} {}", method, pattern)));
            if pattern
                .split('/')
                .any(|part| part.starts_with(':') && !matches!(part, ":version" | ":revision"))
            {
                assert!(records[0].resource.contains(&id), "{line}");
            }
        }
        count += 1;
    }
    assert_eq!(count, 224);
}
