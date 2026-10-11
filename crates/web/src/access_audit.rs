//! Durable outcomes for recognized application requests.

use application::{
    access_audit::{AccessAuditRecord, AccessAuditWriter},
    audit_context::AuditScope,
};
use axum::{
    extract::{ConnectInfo, MatchedPath, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Router,
};
use domain::audit::{AuditContext, AuditSource};
use std::{net::SocketAddr, sync::Arc, time::Instant};

tokio::task_local! { static REQUEST_SCOPE: AuditScope; }
mod drain;
pub use drain::AccessAuditDrain;

/// Forwarding is trusted only when explicitly enabled and the socket is local.
#[derive(Clone, Copy)]
pub enum AuditProxyPolicy {
    Direct,
    LoopbackProxy,
}

#[derive(Clone)]
struct Config {
    writer: Arc<dyn AccessAuditWriter>,
    proxy: AuditProxyPolicy,
    drain: AccessAuditDrain,
}

/// Attach the durable audit writer to the existing bounded HTTP admission layer.
pub fn with_access_audit(
    router: Router,
    writer: Arc<dyn AccessAuditWriter>,
    proxy: AuditProxyPolicy,
) -> Router {
    with_access_audit_drain(router, writer, proxy, AccessAuditDrain::default())
}

pub fn with_access_audit_drain(
    router: Router,
    writer: Arc<dyn AccessAuditWriter>,
    proxy: AuditProxyPolicy,
    drain: AccessAuditDrain,
) -> Router {
    router.layer(Extension(Config {
        writer,
        proxy,
        drain,
    }))
}

pub(crate) fn reserve(request: &Request) -> Option<drain::Admission> {
    let config = request.extensions().get::<Config>()?;
    request
        .extensions()
        .get::<MatchedPath>()?
        .as_str()
        .starts_with("/api/v1/")
        .then(|| config.drain.reserve())
}

pub(crate) fn current_scope() -> Option<AuditScope> {
    REQUEST_SCOPE.try_with(Clone::clone).ok()
}

pub(crate) async fn execute(request: Request, next: Next) -> Response {
    let Some(config) = request.extensions().get::<Config>().cloned() else {
        return next.run(request).await;
    };
    let Some(path) = request.extensions().get::<MatchedPath>() else {
        return next.run(request).await;
    };
    if !path.as_str().starts_with("/api/v1/") {
        return next.run(request).await;
    }
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|value| value.0.ip());
    let forwarded = if matches!(config.proxy, AuditProxyPolicy::LoopbackProxy)
        && peer.is_some_and(|ip| ip.is_loopback())
    {
        let mut values = request.headers().get_all("x-forwarded-for").iter();
        let first = values
            .next()
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok());
        if values.next().is_none() {
            first
        } else {
            None
        }
    } else {
        None
    };
    let scope = AuditScope::new(AuditContext {
        user_id: None,
        ip_address: forwarded.or(peer),
        source: AuditSource::Http,
        request_id: Some(
            request
                .extensions()
                .get::<crate::access_log::AccessAttempt>()
                .map_or_else(uuid::Uuid::new_v4, |attempt| attempt.request_id()),
        ),
    });
    let resource = resource(&request, path.as_str());
    let operation = resource.split(' ').take(2).collect::<Vec<_>>().join(" ");
    REQUEST_SCOPE
        .scope(scope.clone(), async move {
            let response = next.run(request).await;
            let context = match scope.snapshot() {
                Ok(context) => context,
                Err(_) => return crate::error::ApiError::internal().into_response(),
            };
            let request_id = context.request_id;
            let record = AccessAuditRecord {
                context,
                action: outcome(response.status()).into(),
                resource,
            };
            // The timer includes waiting for the blocking pool, writer lock and commit.
            let start = Instant::now();
            let written = tokio::task::spawn_blocking(move || config.writer.record(record)).await;
            let durable = matches!(written, Ok(Ok(())));
            tracing::info!(target: "qadra::audit_commit", durable,
            request_id = ?request_id, operation,
            elapsed_micros = start.elapsed().as_micros() as u64, "access audit append finished");
            if durable {
                response
            } else {
                crate::error::ApiError::internal().into_response()
            }
        })
        .await
}

fn outcome(status: StatusCode) -> &'static str {
    if status.is_success() {
        "access.granted"
    } else if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        "access.denied"
    } else if status.is_server_error() {
        "access.failed"
    } else {
        "access.rejected"
    }
}

fn resource(request: &Request, pattern: &str) -> String {
    let method = match request.method().as_str() {
        "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" => {
            request.method().as_str()
        }
        _ => "OTHER",
    };
    let mut value = format!("{} {}", method, pattern);
    for (part, actual) in pattern.split('/').zip(request.uri().path().split('/')) {
        if let Some(name) = part.strip_prefix(':') {
            let id = uuid::Uuid::parse_str(actual)
                .ok()
                .map(|id| id.to_string())
                .or_else(|| {
                    actual
                        .parse::<u64>()
                        .ok()
                        .filter(|_| actual.len() <= 20)
                        .map(|id| id.to_string())
                });
            if let Some(id) = id {
                value.push_str(&format!(" {name}={id}"));
            }
        }
    }
    value
}

#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod drain_tests;
