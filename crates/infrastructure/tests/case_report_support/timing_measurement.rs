use super::super::store_url;
use crate::case_report_support::{command, request};
use application::{case_reports::*, identity::Principal};
use domain::clock::Clock;
use infrastructure::{RingSha256Hasher, SystemClock};
use postgres::{Client, NoTls};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc, time::Duration as Wait};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use uuid::Uuid;

#[path = "timing_observation.rs"]
mod observation;
use observation::ObservedRenderer;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Scenario {
    Single,
    Queue,
    Retry,
}

impl Scenario {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Queue => "queue",
            Self::Retry => "transient_retry",
        }
    }
}

pub(super) struct Measurement {
    pub(super) row: Value,
    pub(super) estimated_ms: Option<u64>,
    pub(super) actual_ms: Option<f64>,
    pub(super) ready: bool,
    pub(super) capacity_failure: bool,
}

pub(super) fn measure(
    admin_url: &str,
    runtime_url: &str,
    actor: &Principal,
    period: OffsetDateTime,
    kind: CaseReportKind,
    executable: &Path,
    scenario: Scenario,
) -> Measurement {
    let mut admin = Client::connect(admin_url, NoTls).unwrap();
    let store = store_url(runtime_url);
    let renderer = Arc::new(ObservedRenderer::new(
        executable,
        scenario == Scenario::Retry,
    ));
    let worker = CaseReportWorker::new(
        store.clone(),
        renderer.clone(),
        Arc::new(RingSha256Hasher),
        Arc::new(SystemClock),
    );
    let enqueue = || {
        let mut command = command(period - Duration::days(1));
        command.filters.kind = kind;
        request(store.as_ref(), actor, command, SystemClock.now()).unwrap()
    };
    let requested = if scenario == Scenario::Queue {
        let mut earliest = enqueue().id;
        let mut target = None;
        for _ in 0..16 {
            let candidate = enqueue();
            if earliest < candidate.id {
                target = Some(candidate);
                break;
            }
            earliest = candidate.id;
        }
        target.expect("normal UUID requests must produce a target with a queued predecessor")
    } else {
        enqueue()
    };
    // Capture the original prediction before any consumer or rendering attempt.
    let estimated_ms = requested.estimate.map(|value| value.total_ms());
    let queue_ids: Vec<Uuid> = admin
        .query(
            "SELECT id FROM case_report_jobs WHERE state='queued' ORDER BY id",
            &[],
        )
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    let ahead = queue_ids
        .iter()
        .position(|id| *id == requested.id.as_uuid())
        .unwrap();
    assert_eq!(ahead > 0, scenario == Scenario::Queue);
    let mut trace = Vec::new();
    let mut consumed = Vec::new();
    let mut pause = Wait::from_secs(1);
    let mut result = requested.clone();
    let mut worker_error = false;
    let mut retry_seen = false;
    let mut retry_wait_ms = 0.0;
    for _ in 0..64 {
        std::thread::sleep(pause);
        let run = worker.run_next();
        match run {
            Ok(CaseReportWorkerRun::Ready(id)) => {
                consumed.push(id.as_uuid());
                trace.push(json!({"state":"ready", "id":id.to_string()}));
            }
            Ok(CaseReportWorkerRun::Deferred(id)) => {
                retry_seen = true;
                trace.push(json!({"state":"retry", "id":id.to_string()}));
            }
            Ok(CaseReportWorkerRun::Failed(id)) => {
                trace.push(json!({"state":"failed", "id":id.to_string()}))
            }
            Ok(CaseReportWorkerRun::AccessRevoked(id)) => {
                trace.push(json!({"state":"revoked", "id":id.to_string()}))
            }
            Ok(CaseReportWorkerRun::Idle) => trace.push(json!({"state":"idle"})),
            Err(_) => {
                worker_error = true;
                trace.push(json!({"state":"worker_error"}));
            }
        }
        result = store.get(actor, requested.id, SystemClock.now()).unwrap();
        assert_eq!(result.estimate, requested.estimate);
        if worker_error
            || matches!(
                result.state,
                CaseReportState::Ready { .. }
                    | CaseReportState::Failed(_)
                    | CaseReportState::AccessRevoked
            )
        {
            break;
        }
        pause = match result.state {
            CaseReportState::RetryWaiting { retry_at, .. } => {
                let remaining = (retry_at - SystemClock.now()).as_seconds_f64().max(1.0);
                retry_wait_ms += remaining * 1000.0;
                Wait::from_secs_f64(remaining)
            }
            _ => Wait::from_secs(1),
        };
    }
    let ready = matches!(result.state, CaseReportState::Ready { .. });
    let capacity_failure = matches!(
        result.state,
        CaseReportState::Failed(CaseReportFailure::CapacityExceeded)
    );
    let actual_ms = result
        .notice
        .as_ref()
        .filter(|notice| ready && notice.kind == CaseReportNoticeKind::Ready)
        .map(|notice| (notice.created_at - requested.requested_at).as_seconds_f64() * 1000.0);
    let queue_order_matches = consumed.as_slice() == &queue_ids[..=ahead];
    let retry_requirement_satisfied = scenario != Scenario::Retry
        || (retry_seen && actual_ms.is_some_and(|actual| actual >= 30_000.0));
    let snapshot_bytes = admin
        .query_opt(
            "SELECT octet_length(payload)-28 FROM case_report_snapshots WHERE report_id=$1",
            &[&requested.id.as_uuid()],
        )
        .unwrap()
        .map(|row| row.get::<_, i32>(0));
    let samples = renderer.samples();
    let matching_samples: Vec<_> = samples
        .iter()
        .filter(|sample| sample["report_id"] == requested.id.to_string())
        .cloned()
        .collect();
    let initial_id = requested.id.to_string();
    let row = json!({
        "report_id": initial_id,
        "scenario": scenario.name(),
        "estimated_ms": estimated_ms,
        "actual_ms": actual_ms,
        "tolerance_ms": actual_ms.map(|actual| 2000.0f64.max(0.30 * actual)),
        "within_tolerance": estimated_ms.zip(actual_ms).map(|(estimated, actual)|
            (estimated as f64 - actual).abs() <= 2000.0f64.max(0.30 * actual)),
        "ready": ready,
        "state": state_name(&result.state),
        "worker_error": worker_error,
        "requested_at": requested.requested_at.format(&Rfc3339).unwrap(),
        "ready_at": result.notice.as_ref().filter(|notice| ready && notice.kind == CaseReportNoticeKind::Ready)
            .map(|notice| notice.created_at.format(&Rfc3339).unwrap()),
        "queued_predecessors": ahead,
        "queue_ids": queue_ids.iter().map(Uuid::to_string).collect::<Vec<_>>(),
        "queue_order_matches": queue_order_matches,
        "retry_seen": retry_seen,
        "retry_requirement_satisfied": retry_requirement_satisfied,
        "scheduled_retry_wait_ms": retry_wait_ms,
        "worker_trace": trace,
        "snapshot_plaintext_bytes": snapshot_bytes,
        "renders": matching_samples,
        "all_render_attempts": samples,
        "initial_consumer_pause_ms": 1000,
        "inter_job_pause_ms": 1000,
        "transient_failure": if scenario == Scenario::Retry { Some("one injected RenderUnavailable; subsequent renders use isolated executable") } else { None },
    });
    Measurement {
        row,
        estimated_ms,
        actual_ms,
        ready,
        capacity_failure,
    }
}

fn state_name(state: &CaseReportState) -> &'static str {
    match state {
        CaseReportState::Queued => "queued",
        CaseReportState::Processing(_) => "processing",
        CaseReportState::RetryWaiting { .. } => "retry_waiting",
        CaseReportState::Ready { .. } => "ready",
        CaseReportState::Failed(CaseReportFailure::CapacityExceeded) => "capacity_exceeded",
        CaseReportState::Failed(_) => "failed",
        CaseReportState::AccessRevoked => "access_revoked",
    }
}
