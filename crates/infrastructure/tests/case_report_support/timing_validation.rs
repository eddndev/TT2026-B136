use super::seeded_fixture;
use crate::case_report_email::backup::RestoredDatabase;
use crate::case_report_support::{owner, Fixture};
use application::{case_reports::CaseReportKind, identity::Principal};
use domain::crypto::DocumentHasher;
use infrastructure::{case_report_isolation::IsolatedCaseReportRenderer, RingSha256Hasher};
use postgres::{Client, NoTls};
use serde_json::{json, Value};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

#[path = "timing_measurement.rs"]
mod measurement;
use measurement::{measure, Measurement, Scenario};

const KINDS: [CaseReportKind; 2] = [CaseReportKind::CaseState, CaseReportKind::LitigatorActivity];
const VALIDATION_SAMPLES: usize = 28;

struct Baseline {
    db: Fixture,
    actor: Principal,
    cases: usize,
    history: Value,
    history_digest: String,
}

#[test]
#[ignore = "manual isolated-renderer validation requires PostgreSQL, pg_dump, pg_restore and TT_REPORT_TIMING_EXECUTABLE"]
fn isolated_report_estimates_validate_against_frozen_calibration_history() {
    let executable =
        PathBuf::from(std::env::var_os("TT_REPORT_TIMING_EXECUTABLE").expect(
            "TT_REPORT_TIMING_EXECUTABLE must name the current absolute report-worker binary",
        ));
    IsolatedCaseReportRenderer::new(executable.clone()).unwrap();
    let binary_digest = RingSha256Hasher
        .hash_stream(&mut File::open(&executable).unwrap())
        .unwrap()
        .to_hex();
    println!(
        "REPORT_VALIDATION_PLAN {}",
        json!({
            "renderer": "IsolatedCaseReportRenderer",
            "renderer_executable": executable,
            "renderer_sha256": binary_digest,
            "requested_volumes": [3, 80, 900],
            "capacity_only_high_volume_fallback": 400,
            "calibration_per_kind_and_volume": 3,
            "validation_per_kind_and_volume": 4,
            "additional_per_kind_at_80_cases": ["queue", "transient_retry"],
            "validation_requests": VALIDATION_SAMPLES,
            "accuracy_rule": "abs(estimated_ms-actual_ms)<=max(2000,0.30*actual_ms)",
            "required_accuracy_percent": 90,
            "history_policy": "finish all calibration before validation; restore the unchanged baseline separately for every validation request",
            "activity_source": "original document uploads only; no procedural acts or attended deadlines",
        })
    );
    let mut baselines = Vec::new();
    for cases in [3, 80] {
        baselines.push(
            calibrate(cases, &executable)
                .expect("small and medium calibration must publish both formats"),
        );
    }
    let high = match calibrate(900, &executable) {
        Some(value) => value,
        None => {
            println!(
                "REPORT_CALIBRATION_DECISION {}",
                json!({
                    "requested_cases": 900, "selected_cases": 400,
                    "reason": "isolated renderer capacity failure before any validation request",
                })
            );
            calibrate(400, &executable).expect("fallback calibration must publish both formats")
        }
    };
    baselines.push(high);
    println!(
        "REPORT_VALIDATION_FROZEN {}",
        json!({
            "volumes": baselines.iter().map(|value| value.cases).collect::<Vec<_>>(),
            "history": baselines.iter().map(|value| json!({
                "cases":value.cases, "sha256":value.history_digest, "samples":value.history,
            })).collect::<Vec<_>>(),
            "renderer_sha256": binary_digest,
        })
    );
    let mut results = Vec::new();
    for baseline in &mut baselines {
        for kind in KINDS {
            for sample in 1..=4 {
                results.push(validate(
                    baseline,
                    kind,
                    sample,
                    Scenario::Single,
                    &executable,
                ));
            }
        }
    }
    for kind in KINDS {
        for scenario in [Scenario::Queue, Scenario::Retry] {
            results.push(validate(&mut baselines[1], kind, 1, scenario, &executable));
        }
    }
    for baseline in &mut baselines {
        assert_eq!(
            history(&mut baseline.db.admin),
            baseline.history,
            "validation must never add training observations to a source baseline"
        );
    }
    assert_eq!(results.len(), VALIDATION_SAMPLES);
    let estimated = results
        .iter()
        .filter(|value| value.estimated_ms.is_some())
        .count();
    let ready = results.iter().filter(|value| value.ready).count();
    let valid_consumer_semantics = results
        .iter()
        .filter(|value| {
            value.row["queue_order_matches"] == true
                && value.row["retry_requirement_satisfied"] == true
        })
        .count();
    let passed = results
        .iter()
        .filter(|value| {
            value
                .estimated_ms
                .zip(value.actual_ms)
                .is_some_and(|(estimate, actual)| {
                    (estimate as f64 - actual).abs() <= 2000.0f64.max(0.30 * actual)
                })
        })
        .count();
    let accuracy = (estimated > 0).then(|| 100.0 * passed as f64 / estimated as f64);
    let aggregate = json!({
        "requests": VALIDATION_SAMPLES,
        "rows": results.iter().map(|value| &value.row).collect::<Vec<_>>(),
        "numerator_within_tolerance": passed,
        "denominator_with_initial_estimate": estimated,
        "denominator_policy": "all initial estimates, including failures without readiness; failures cannot count as accurate",
        "without_estimate": VALIDATION_SAMPLES - estimated,
        "coverage": estimated as f64 / VALIDATION_SAMPLES as f64,
        "coverage_percent": 100.0 * estimated as f64 / VALIDATION_SAMPLES as f64,
        "accuracy_percent": accuracy,
        "ready": ready,
        "not_ready": VALIDATION_SAMPLES - ready,
        "valid_consumer_semantics": valid_consumer_semantics,
        "renderer_sha256": binary_digest,
        "activity_source": "original document uploads only; no procedural acts or attended deadlines",
    });
    println!("REPORT_VALIDATION {aggregate}");
    assert_eq!(
        ready, VALIDATION_SAMPLES,
        "validation includes non-ready requests"
    );
    assert_eq!(
        valid_consumer_semantics, VALIDATION_SAMPLES,
        "validation must preserve UUID ordering and real retry delay"
    );
    assert!(
        estimated > 0 && passed * 100 >= estimated * 90,
        "fixed estimator did not meet 90 percent accuracy; retain every failed observation"
    );
}

fn calibrate(cases: usize, executable: &Path) -> Option<Baseline> {
    let mut db = seeded_fixture(cases);
    let actor = owner(&mut db);
    for kind in KINDS {
        for sample in 1..=3 {
            let mut result = measure(
                &db.admin_url,
                &db.runtime_url,
                &actor,
                db.at,
                kind,
                executable,
                Scenario::Single,
            );
            annotate(&mut result.row, "calibration", cases, kind, sample);
            println!("REPORT_CALIBRATION {}", result.row);
            if result.capacity_failure {
                return None;
            }
            assert!(
                result.ready,
                "calibration failed outside renderer capacity limits"
            );
        }
    }
    let history = history(&mut db.admin);
    assert_eq!(history.as_array().unwrap().len(), 6);
    let history_digest = RingSha256Hasher
        .hash_bytes(&serde_json::to_vec(&history).unwrap())
        .to_hex();
    Some(Baseline {
        db,
        actor,
        cases,
        history,
        history_digest,
    })
}

fn validate(
    baseline: &mut Baseline,
    kind: CaseReportKind,
    sample: usize,
    scenario: Scenario,
    executable: &Path,
) -> Measurement {
    let restored = RestoredDatabase::copy(&baseline.db);
    let mut admin = Client::connect(&restored.admin_url, NoTls).unwrap();
    assert_eq!(
        history(&mut admin),
        baseline.history,
        "each validation request must start from the frozen calibration history"
    );
    drop(admin);
    let mut result = measure(
        &restored.admin_url,
        &restored.runtime_url,
        &baseline.actor,
        baseline.db.at,
        kind,
        executable,
        scenario,
    );
    annotate(&mut result.row, "validation", baseline.cases, kind, sample);
    result.row["baseline_history_sha256"] = json!(baseline.history_digest);
    result.row["baseline_samples_per_kind"] = json!(3);
    result.row["fresh_database_copy"] = json!(true);
    println!("REPORT_VALIDATION_SAMPLE {}", result.row);
    drop(restored);
    result
}

fn annotate(row: &mut Value, phase: &str, cases: usize, kind: CaseReportKind, sample: usize) {
    row["phase"] = json!(phase);
    row["cases"] = json!(cases);
    row["sample"] = json!(sample);
    row["kind"] = json!(match kind {
        CaseReportKind::CaseState => "case_state",
        CaseReportKind::LitigatorActivity => "litigator_activity",
    });
    row["fixture"] = json!(format!("original-uploads-{cases}-cases-3-authors"));
}

fn history(admin: &mut Client) -> Value {
    admin
        .query_one(
            "SELECT coalesce(jsonb_agg(jsonb_build_object('id',j.id,'scope',j.scope,
        'kind',coalesce(j.command->'filters'->>'kind','case_state'),
        'cases',cardinality(j.case_ids),'checked_at',s.checked_at,'ready_at',n.created_at)
        ORDER BY j.id),'[]'::jsonb)
        FROM case_report_jobs j JOIN case_report_snapshots s ON s.report_id=j.id
        JOIN case_report_notices n ON n.report_id=j.id WHERE j.state='ready' AND n.kind='ready'",
            &[],
        )
        .unwrap()
        .get(0)
}
