use application::{case_reports::*, ApplicationError};
use infrastructure::case_report_isolation::IsolatedCaseReportRenderer;
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Instant,
};

pub(super) struct ObservedRenderer {
    inner: IsolatedCaseReportRenderer,
    fail_once: AtomicBool,
    samples: Mutex<Vec<Value>>,
}

impl ObservedRenderer {
    pub(super) fn new(executable: &Path, transient_failure: bool) -> Self {
        Self {
            inner: IsolatedCaseReportRenderer::new(executable.to_path_buf()).unwrap(),
            fail_once: AtomicBool::new(transient_failure),
            samples: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn samples(&self) -> Vec<Value> {
        self.samples.lock().unwrap().clone()
    }
}

impl CaseReportRenderer for ObservedRenderer {
    fn render(
        &self,
        snapshot: &CaseReportSnapshot,
        format: CaseReportFormat,
    ) -> Result<Vec<u8>, ApplicationError> {
        let size = capture_size(snapshot);
        let started = Instant::now();
        let injected = self.fail_once.swap(false, Ordering::SeqCst);
        let result = if injected {
            Err(CaseReportError::RenderUnavailable.into())
        } else {
            self.inner.render(snapshot, format)
        };
        self.samples.lock().unwrap().push(json!({
            "report_id": snapshot.report_id.to_string(),
            "format": match format { CaseReportFormat::Pdf => "pdf", CaseReportFormat::Csv => "csv" },
            "render_ms": started.elapsed().as_secs_f64() * 1000.0,
            "bytes": result.as_ref().ok().map(Vec::len),
            "injected_render_unavailable": injected,
            "succeeded": result.is_ok(),
            "capture": size,
        }));
        result
    }
}

fn capture_size(snapshot: &CaseReportSnapshot) -> Value {
    let assignments: usize = snapshot
        .cases
        .iter()
        .map(|case| case.assigned_litigators.len())
        .sum();
    let case_text_bytes: usize = snapshot
        .cases
        .iter()
        .map(|case| case.title.len() + case.reference.len())
        .sum();
    let assigned_email_bytes: usize = snapshot
        .cases
        .iter()
        .flat_map(|case| &case.assigned_litigators)
        .map(|actor| actor.email.len())
        .sum();
    let workload_email_bytes: usize = snapshot
        .workload
        .iter()
        .map(|row| row.litigator.email.len())
        .sum();
    let actor_email_bytes: usize = snapshot.activity.as_ref().map_or(0, |activity| {
        activity.actors.iter().map(|actor| actor.email.len()).sum()
    });
    let uploads: u64 = snapshot.activity.as_ref().map_or(0, |activity| {
        assert!(activity.documents_complete);
        assert!(activity
            .rows
            .iter()
            .all(|row| row.procedural_activities == 0 && row.deadlines_attended == 0));
        activity.rows.iter().map(|row| row.documents_uploaded).sum()
    });
    json!({
        "cases": snapshot.cases.len(),
        "rows": snapshot.activity.as_ref().map_or(snapshot.cases.len(), |value| value.rows.len()),
        "assignments": assignments,
        "actors": snapshot.activity.as_ref().map_or(snapshot.workload.len(), |value| value.actors.len()),
        "original_uploads": uploads,
        "case_title_reference_bytes": case_text_bytes,
        "max_case_title_bytes": snapshot.cases.iter().map(|case| case.title.len()).max().unwrap_or(0),
        "max_case_reference_bytes": snapshot.cases.iter().map(|case| case.reference.len()).max().unwrap_or(0),
        "captured_email_bytes": assigned_email_bytes + workload_email_bytes + actor_email_bytes,
    })
}
