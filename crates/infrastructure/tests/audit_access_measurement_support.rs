use application::access_audit::{AccessAuditRecord, AccessAuditService, AccessAuditWriter};
use domain::{audit::AuditContext, identity::UserId};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    sync::{mpsc, Arc, Barrier, Mutex},
    time::Instant,
};
use uuid::Uuid;

pub const OUTCOMES: [&str; 4] = [
    "access.granted",
    "access.denied",
    "access.rejected",
    "access.failed",
];
pub const REPETITIONS: usize = 3;
pub const HISTORY: usize = 1000;
pub const LIMIT_NS: u64 = 500_000_000;

#[derive(Clone, Copy)]
pub struct Route {
    pub method: &'static str,
    pub path: &'static str,
    pub policy: &'static str,
}

pub fn catalog() -> Vec<Route> {
    let document = include_str!("../../../docs/access-audit-matrix.md");
    let block = document
        .split_once("```tsv\n")
        .unwrap()
        .1
        .split("```")
        .next()
        .unwrap();
    let routes: Vec<_> = block
        .lines()
        .skip(1)
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 4, "invalid versioned access catalog row");
            assert!(matches!(fields[0], "GET" | "POST" | "PUT" | "DELETE"));
            Route {
                method: fields[0],
                path: fields[1],
                policy: fields[2],
            }
        })
        .collect();
    assert_eq!(routes.len(), 222, "access measurement catalog changed");
    assert_eq!(
        routes
            .iter()
            .map(|r| (r.method, r.path))
            .collect::<BTreeSet<_>>()
            .len(),
        222
    );
    routes
}

#[derive(Serialize)]
pub struct Sample {
    pub index: usize,
    pub repetition: usize,
    pub concurrency: usize,
    pub writer_phase: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub policy: &'static str,
    pub action: &'static str,
    pub context: AuditContext,
    pub queued_at_ns: u64,
    pub worker_started_at_ns: u64,
    pub returned_at_ns: u64,
    pub elapsed_ns: u64,
    pub worker_queue_ns: u64,
    pub durable: bool,
    pub failure: Option<&'static str>,
    pub audit_sequence: Option<u64>,
    pub audit_chain: Option<String>,
}

struct Job {
    index: usize,
    repetition: usize,
    route: Route,
    action: &'static str,
    context: AuditContext,
    queued: Instant,
    cold: bool,
}

fn elapsed_ns(start: Instant, end: Instant) -> u64 {
    u64::try_from(end.duration_since(start).as_nanos()).unwrap()
}

pub fn measure(
    service: Arc<AccessAuditService>,
    owner: UserId,
    concurrency: usize,
    routes: &[Route],
) -> Vec<Sample> {
    let count = routes.len() * OUTCOMES.len() * REPETITIONS;
    let population: Vec<_> = (1..=REPETITIONS)
        .flat_map(|repetition| {
            routes
                .iter()
                .flat_map(move |route| OUTCOMES.map(|action| (repetition, *route, action)))
        })
        .collect();
    let (send, receive) = mpsc::sync_channel::<Job>(concurrency);
    let receive = Arc::new(Mutex::new(receive));
    let (finished, completed) = mpsc::channel();
    let ready = Arc::new(Barrier::new(concurrency + 1));
    let epoch = Instant::now();
    let mut samples = Vec::with_capacity(count);
    std::thread::scope(|threads| {
        for _ in 0..concurrency {
            let (receive, finished, service, ready) = (
                receive.clone(),
                finished.clone(),
                service.clone(),
                ready.clone(),
            );
            threads.spawn(move || {
                ready.wait();
                loop {
                    let job = { receive.lock().unwrap().recv() };
                    let Ok(job) = job else { break };
                    let record = AccessAuditRecord {
                        context: job.context.clone(),
                        action: job.action.to_owned(),
                        resource: format!("{} {}", job.route.method, job.route.path),
                    };
                    let started = Instant::now();
                    let result = service.record(record);
                    let returned = Instant::now();
                    finished
                        .send(Sample {
                            index: job.index,
                            repetition: job.repetition,
                            concurrency,
                            writer_phase: if job.cold {
                                "fresh_writer_first_batch"
                            } else {
                                "warm_writer"
                            },
                            method: job.route.method,
                            path: job.route.path,
                            policy: job.route.policy,
                            action: job.action,
                            context: job.context,
                            queued_at_ns: elapsed_ns(epoch, job.queued),
                            worker_started_at_ns: elapsed_ns(epoch, started),
                            returned_at_ns: elapsed_ns(epoch, returned),
                            elapsed_ns: elapsed_ns(job.queued, returned),
                            worker_queue_ns: elapsed_ns(job.queued, started),
                            durable: result.is_ok(),
                            failure: result.err().map(|_| "append_failed"),
                            audit_sequence: None,
                            audit_chain: None,
                        })
                        .unwrap();
                }
            });
        }
        ready.wait();
        for (batch, chunk) in population.chunks(concurrency).enumerate() {
            for (offset, (repetition, route, action)) in chunk.iter().enumerate() {
                let context = AuditContext {
                    user_id: if *action == "access.denied" {
                        None
                    } else {
                        Some(owner)
                    },
                    ip_address: Some("192.0.2.80".parse().unwrap()),
                    source: domain::audit::AuditSource::Http,
                    request_id: Some(Uuid::new_v4()),
                };
                let queued = Instant::now();
                send.send(Job {
                    index: batch * concurrency + offset,
                    repetition: *repetition,
                    route: *route,
                    action,
                    context,
                    queued,
                    cold: batch == 0,
                })
                .unwrap();
            }
            for _ in chunk {
                samples.push(
                    completed
                        .recv_timeout(std::time::Duration::from_secs(30))
                        .expect("native access worker did not return within its watchdog"),
                );
            }
        }
        drop(send);
    });
    samples.sort_by_key(|sample| sample.index);
    assert_eq!(samples.len(), count);
    samples
}

pub fn distribution(samples: &[Sample]) -> serde_json::Value {
    let mut durations: Vec<_> = samples
        .iter()
        .filter(|s| s.durable)
        .map(|s| s.elapsed_ns)
        .collect();
    durations.sort_unstable();
    let percentile = |percent: usize| {
        let rank = (durations.len() * percent).div_ceil(100);
        durations.get(rank.saturating_sub(1)).copied()
    };
    serde_json::json!({
        "samples": samples.len(),
        "durable_samples": durations.len(),
        "failed_samples": samples.iter().filter(|s| !s.durable).count(),
        "fresh_writer_first_batch_samples": samples.iter().filter(|s| s.writer_phase == "fresh_writer_first_batch").count(),
        "warm_writer_samples": samples.iter().filter(|s| s.writer_phase == "warm_writer").count(),
        "over_or_equal_limit": samples.iter().filter(|s| s.durable && s.elapsed_ns >= LIMIT_NS).count(),
        "minimum_ns": durations.first(), "maximum_ns": durations.last(),
        "p50_ns": percentile(50), "p95_ns": percentile(95), "p99_ns": percentile(99),
        "quantile_method": "nearest rank: sorted[ceil(p*n)-1]",
        "strict_limit_ns": LIMIT_NS,
    })
}
