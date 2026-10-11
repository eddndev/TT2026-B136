use crate::case_report_support::{command, owner, request, seed_case, Fixture};
use application::{
    case_reports::*,
    documents::{CaseDocumentStore, DocumentRecord},
    ApplicationError,
};
use domain::{
    clock::Clock,
    crypto::{DocumentHasher, DocumentId, DocumentVersion},
};
use infrastructure::{
    case_report_rendering::BoundedCaseReportRenderer, CaseReportEnvelopeProtector,
    EnvelopeKeyManager, PostgresCaseDocumentStore, PostgresCaseReportStore, RingAesGcmCipher,
    RingSha256Hasher, SystemClock,
};
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CaptureSize {
    cases: usize,
    rows: usize,
    assignments: usize,
    actors: usize,
    uploads: u64,
}

struct RenderTiming {
    format: CaseReportFormat,
    bytes: usize,
    elapsed_ms: f64,
    size: CaptureSize,
}

struct MeasuredRenderer {
    inner: BoundedCaseReportRenderer,
    samples: Mutex<Vec<RenderTiming>>,
}

impl CaseReportRenderer for MeasuredRenderer {
    fn render(
        &self,
        snapshot: &CaseReportSnapshot,
        format: CaseReportFormat,
    ) -> Result<Vec<u8>, ApplicationError> {
        let started = Instant::now();
        let content = self.inner.render(snapshot, format)?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let size = CaptureSize {
            cases: snapshot.cases.len(),
            rows: snapshot
                .activity
                .as_ref()
                .map_or(snapshot.cases.len(), |activity| activity.rows.len()),
            assignments: snapshot
                .cases
                .iter()
                .map(|case| case.assigned_litigators.len())
                .sum(),
            actors: snapshot
                .activity
                .as_ref()
                .map_or(snapshot.workload.len(), |activity| activity.actors.len()),
            uploads: snapshot.activity.as_ref().map_or(0, |activity| {
                assert!(activity.documents_complete);
                assert!(activity
                    .rows
                    .iter()
                    .all(|row| { row.procedural_activities == 0 && row.deadlines_attended == 0 }));
                activity.rows.iter().map(|row| row.documents_uploaded).sum()
            }),
        };
        self.samples.lock().unwrap().push(RenderTiming {
            format,
            bytes: content.len(),
            elapsed_ms,
            size,
        });
        Ok(content)
    }
}

fn microsecond_now() -> OffsetDateTime {
    let now = SystemClock.now();
    now.replace_nanosecond(now.nanosecond() / 1000 * 1000)
        .unwrap()
}

fn seeded_fixture(cases: usize) -> Fixture {
    let mut db = Fixture::new()
        .expect("manual report timing requires an isolated backend in CASE_TEST_DATABASE_URL");
    db.at = microsecond_now();
    let created_at = db.at;
    let mut ids = vec![db.case];
    for index in 1..cases {
        ids.push(seed_case(
            &mut db,
            &format!("Timing case {cases:03}-{index:03}"),
            created_at,
        ));
    }
    let authors = [
        db.user("litigator", false),
        db.user("litigator", false),
        db.user("litigator", false),
    ];
    let documents = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
    for (index, case_id) in ids.into_iter().enumerate() {
        for author in authors {
            db.admin
                .execute(
                    "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                    &[&case_id.as_uuid(), &author.as_uuid()],
                )
                .unwrap();
        }
        // Use the existing original-upload fixture path, not fabricated legal acts.
        let document = DocumentRecord::pending(
            DocumentId::new(),
            DocumentVersion::initial(),
            format!("timing-evidence-{index:03}.txt"),
            RingSha256Hasher.hash_bytes(b"report timing upload fixture"),
            vec![8; 80],
        )
        .unwrap();
        documents
            .insert(
                authors[index % authors.len()],
                case_id,
                document,
                microsecond_now(),
            )
            .unwrap();
    }
    db
}

fn store(db: &Fixture) -> Arc<PostgresCaseReportStore> {
    store_url(&db.runtime_url)
}

fn store_url(runtime_url: &str) -> Arc<PostgresCaseReportStore> {
    let protector = CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        vec![0x52; 32],
    )
    .unwrap();
    Arc::new(
        PostgresCaseReportStore::open(
            runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(SystemClock),
            Arc::new(protector),
        )
        .unwrap(),
    )
}

fn event_at(db: &mut Fixture, report: &CaseReportDetail, action: &str) -> OffsetDateTime {
    let resource = format!(
        "case-report:{}:request:{}",
        report.id, report.request_digest
    );
    let rows = db
        .admin
        .query(
            "SELECT timestamp FROM audit_events WHERE action=$1 AND resource=$2",
            &[&action, &resource],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    OffsetDateTime::parse(rows[0].get::<_, &str>(0), &Rfc3339).unwrap()
}

fn elapsed_ms(start: OffsetDateTime, end: OffsetDateTime) -> f64 {
    assert!(end >= start);
    (end - start).as_seconds_f64() * 1000.0
}

fn measure(db: &mut Fixture, cases: usize, kind: CaseReportKind, sample: usize) {
    let store = store(db);
    let actor = owner(db);
    let renderer = Arc::new(MeasuredRenderer {
        inner: BoundedCaseReportRenderer::bundled().unwrap(),
        samples: Mutex::new(Vec::new()),
    });
    let worker = CaseReportWorker::new(
        store.clone(),
        renderer.clone(),
        Arc::new(RingSha256Hasher),
        Arc::new(SystemClock),
    );
    let mut command = command(db.at - Duration::days(1));
    command.filters.kind = kind;
    let total_started = Instant::now();
    let requested = request(store.as_ref(), &actor, command, SystemClock.now()).unwrap();
    let service_started = Instant::now();
    let result = worker.run_next().unwrap();
    let service_wall_ms = service_started.elapsed().as_secs_f64() * 1000.0;
    let total_wall_ms = total_started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(result, CaseReportWorkerRun::Ready(requested.id));
    let ready = store.get(&actor, requested.id, SystemClock.now()).unwrap();
    let CaseReportState::Ready {
        checked_at,
        artifacts,
        ..
    } = &ready.state
    else {
        panic!("real report rendering did not publish both formats");
    };
    let notice = ready.notice.as_ref().unwrap();
    assert_eq!(notice.kind, CaseReportNoticeKind::Ready);
    let claim_at = event_at(db, &ready, "case_report.claim");
    let captured_at = event_at(db, &ready, "case_report.capture");
    let samples = renderer.samples.lock().unwrap();
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].size, samples[1].size);
    let size = samples[0].size;
    assert_eq!(
        (size.cases, size.rows, size.assignments, size.actors),
        (cases, cases, cases * 3, 3)
    );
    if kind == CaseReportKind::LitigatorActivity {
        assert_eq!(size.uploads, cases as u64);
    }
    let formats: Vec<_> = samples
        .iter()
        .map(|sample| {
            let published = artifacts
                .iter()
                .find(|v| v.format == sample.format)
                .unwrap();
            assert_eq!(published.bytes, sample.bytes as u64);
            json!({
                "format": match sample.format { CaseReportFormat::Pdf => "pdf", CaseReportFormat::Csv => "csv" },
                "bytes": sample.bytes,
                "render_ms": sample.elapsed_ms,
            })
        })
        .collect();
    println!(
        "REPORT_TIMING {}",
        json!({
            "phase": "preliminary",
            "fixture": format!("original-uploads-{cases}-cases-3-authors"),
            "sample": sample,
            "report_id": ready.id.to_string(),
            "kind": match kind { CaseReportKind::CaseState => "case_state", CaseReportKind::LitigatorActivity => "litigator_activity" },
            "rows": size.rows,
            "cases": size.cases,
            "assignments": size.assignments,
            "actors": size.actors,
            "activity_source": "original document uploads only; no procedural acts or attended deadlines",
            "consumer": "direct worker; no supervisor pause or queued predecessor",
            "queue_ms": elapsed_ms(ready.requested_at, claim_at),
            "capture_ms": elapsed_ms(*checked_at, captured_at),
            "service_ms": elapsed_ms(claim_at, notice.created_at),
            "total_ms": elapsed_ms(ready.requested_at, notice.created_at),
            "service_wall_ms": service_wall_ms,
            "total_wall_ms": total_wall_ms,
            "requested_at": ready.requested_at.format(&Rfc3339).unwrap(),
            "ready_at": notice.created_at.format(&Rfc3339).unwrap(),
            "formats": formats,
        })
    );
}

#[test]
#[ignore = "manual calibration requires isolated PostgreSQL and real PDF/CSV rendering"]
fn real_report_generation_timing_by_kind_and_volume() {
    for cases in [3, 80, 400] {
        let mut db = seeded_fixture(cases);
        for kind in [CaseReportKind::CaseState, CaseReportKind::LitigatorActivity] {
            for sample in 1..=3 {
                measure(&mut db, cases, kind, sample);
            }
        }
    }
}

#[path = "timing_validation.rs"]
mod validation;
