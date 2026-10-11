use crate::serve_report_readiness_acceptance_support::*;
use application::case_reports::{
    CaseReportEmailRun, CaseReportEmailState, CaseReportId, CaseReportWorkerRun,
};
use axum::http::{Method, StatusCode};
use domain::audit::{AuditLog, ChainVerification};
use infrastructure::{PostgresAuditLog, RingSha256Hasher};
use serde_json::{json, Value};
use std::{collections::BTreeSet, thread, time::Duration};
use uuid::Uuid;

#[test]
#[ignore = "manual HTTP report acceptance requires CASE_TEST_DATABASE_URL, IDENTITY_TEST_REDIS_URL and TT_REPORT_TIMING_EXECUTABLE"]
fn real_queued_report_delivers_captured_email_and_survives_fresh_mfa() {
    let mut flow = Acceptance::new();
    let token = flow.login(true);
    let mut history = Vec::new();
    for _ in 0..3 {
        let command = command();
        let initial = flow.request(&token, &command).json(StatusCode::ACCEPTED);
        assert_eq!(initial["state"], "queued");
        assert!(initial["estimated_total_ms"].is_null());
        assert!(initial["ready_email"].is_null());
        assert_eq!(flow.email.run_once().unwrap(), CaseReportEmailRun::Idle);
        thread::sleep(Duration::from_secs(1));
        assert_eq!(
            flow.worker.run_next().unwrap(),
            CaseReportWorkerRun::Ready(CaseReportId::from_uuid(id(&initial)))
        );
        let ready = flow
            .get(&format!("{PATH}/{}", id(&initial)), &token)
            .json(StatusCode::OK);
        assert_eq!(ready["state"], "ready");
        assert_eq!(ready["estimated_total_ms"], initial["estimated_total_ms"]);
        assert!(ready["ready_email"].is_null());
        history.push(ready);
    }
    assert!(flow.capture.messages().is_empty());

    let mut pending = Vec::new();
    let mut selected = None;
    for _ in 0..16 {
        let command = command();
        let initial = flow.request(&token, &command).json(StatusCode::ACCEPTED);
        assert_eq!(initial["state"], "queued");
        assert!(initial["ready_email"].is_null());
        let has_predecessor = pending
            .iter()
            .any(|previous: &Value| id(previous) < id(&initial));
        pending.push(initial.clone());
        if has_predecessor {
            assert!(initial["estimated_total_ms"].as_u64().unwrap() > 2000);
            selected = Some((command, initial));
            break;
        }
    }
    let (command, initial) =
        selected.expect("natural report UUIDs must yield a queued predecessor");
    let target = id(&initial);
    let path = format!("{PATH}/{target}");
    assert_eq!(
        flow.request(&token, &command).json(StatusCode::ACCEPTED),
        initial
    );
    assert_eq!(flow.get(&path, &token).json(StatusCode::OK), initial);
    let queue: Vec<Uuid> = flow
        .db
        .admin
        .query(
            "SELECT id FROM case_report_jobs WHERE state='queued' ORDER BY id",
            &[],
        )
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(
        queue.iter().copied().collect::<BTreeSet<_>>(),
        pending.iter().map(id).collect::<BTreeSet<_>>()
    );
    let predecessors = queue.iter().position(|value| *value == target).unwrap();
    assert!(predecessors > 0);
    assert_eq!(flow.email.run_once().unwrap(), CaseReportEmailRun::Idle);
    assert!(flow.capture.messages().is_empty());
    for report_id in &queue {
        thread::sleep(Duration::from_secs(1));
        assert_eq!(
            flow.worker.run_next().unwrap(),
            CaseReportWorkerRun::Ready(CaseReportId::from_uuid(*report_id))
        );
        let current = flow.get(&path, &token).json(StatusCode::OK);
        assert_eq!(current["estimated_total_ms"], initial["estimated_total_ms"]);
        if *report_id < target {
            assert_eq!(current["state"], "queued");
            assert!(current["ready_email"].is_null());
        }
    }
    assert_eq!(flow.worker.run_next().unwrap(), CaseReportWorkerRun::Idle);
    let ready = flow.get(&path, &token).json(StatusCode::OK);
    assert_eq!(ready["state"], "ready");
    assert_eq!(ready["ready_email"], "pending");
    assert!(ready["notice"]["read_at"].is_null());
    assert_eq!(flow.request(&token, &command).json(StatusCode::OK), ready);
    let actual_ms = (instant(&ready["notice"]["created_at"]) - instant(&initial["requested_at"]))
        .as_seconds_f64()
        * 1000.0;
    assert!(actual_ms > 2000.0);
    let original: Vec<_> = ["pdf", "csv"]
        .into_iter()
        .map(|format| {
            (
                format,
                download(
                    flow.get(&format!("{path}/download?format={format}"), &token),
                    &ready,
                    format,
                ),
            )
        })
        .collect();
    assert_eq!(flow.get(&path, &token).json(StatusCode::OK), ready);
    let deliveries: i64 = flow
        .db
        .admin
        .query_one("SELECT count(*) FROM case_report_email_deliveries", &[])
        .unwrap()
        .get(0);
    assert!(deliveries > 0);
    for _ in 0..deliveries {
        assert_eq!(
            flow.email.run_once().unwrap(),
            CaseReportEmailRun::Recorded(CaseReportEmailState::Accepted)
        );
    }
    assert_eq!(flow.email.run_once().unwrap(), CaseReportEmailRun::Idle);
    let captured = flow.capture.messages();
    assert_eq!(captured.len(), deliveries as usize);
    assert_eq!(
        captured
            .iter()
            .map(|message| message.id)
            .collect::<BTreeSet<_>>()
            .len(),
        captured.len()
    );
    let row = flow
        .db
        .admin
        .query_one(
            "SELECT delivery_id,message,state FROM case_report_email_deliveries WHERE report_id=$1",
            &[&target],
        )
        .unwrap();
    let delivery_id: Uuid = row.get("delivery_id");
    let received = captured
        .iter()
        .find(|message| message.id == delivery_id)
        .unwrap();
    let template = template();
    assert_eq!(received.message.from, template.from);
    assert_eq!(received.message.to, flow.owner.principal.email);
    assert_eq!(received.message.subject, template.subject);
    assert_eq!(received.message.text, template.text);
    assert!(received
        .message
        .text
        .contains("https://qadra.example.test/#reports"));
    assert!(received.at >= instant(&ready["notice"]["created_at"]));
    assert_eq!(row.get::<_, &str>("state"), "accepted");
    let envelope = json!({
        "from":received.message.from, "to":received.message.to,
        "subject":received.message.subject, "text":received.message.text,
    });
    assert_eq!(row.get::<_, Value>("message"), envelope);
    let accepted = flow.get(&path, &token).json(StatusCode::OK);
    assert_eq!(accepted["ready_email"], "accepted");
    assert_eq!(stable(&accepted), stable(&ready));

    assert_eq!(
        flow.send(Method::POST, "/api/v1/auth/logout", &token, None)
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(flow.get(&path, &token).status, StatusCode::UNAUTHORIZED);
    flow.reopen();
    let fresh = flow.login(false);
    assert!(
        fresh.as_str() != token.as_str(),
        "reentry retained the revoked session"
    );
    assert_eq!(flow.get(&path, &fresh).json(StatusCode::OK), accepted);
    assert_eq!(
        flow.request(&fresh, &command).json(StatusCode::OK),
        accepted
    );
    for (format, bytes) in original {
        assert_eq!(
            download(
                flow.get(&format!("{path}/download?format={format}"), &fresh),
                &accepted,
                format
            ),
            bytes
        );
    }
    assert_eq!(flow.email.run_once().unwrap(), CaseReportEmailRun::Idle);
    assert_eq!(flow.capture.messages().len(), captured.len());
    let operations: i64 = flow
        .db
        .admin
        .query_one(
            "SELECT count(*) FROM case_report_jobs WHERE operation_id=$1",
            &[&command["operation_id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()],
        )
        .unwrap()
        .get(0);
    assert_eq!(operations, 1);
    let audit = PostgresAuditLog::open(&flow.db.runtime_url).unwrap();
    let entries = audit.load_all().unwrap();
    assert!(
        matches!(domain::audit::verify_chain(&RingSha256Hasher, &entries),
        Ok(ChainVerification::Valid { entries: count }) if count == entries.len())
    );
    for action in [
        "case_report.request",
        "case_report.complete",
        "case_report.email_claim",
        "case_report.email_finish",
    ] {
        assert!(
            entries.iter().any(|entry| entry.event.action == action),
            "missing audit action {action}"
        );
    }
    println!(
        "REPORT_READINESS_HTTP {}",
        json!({
            "renderer":"IsolatedCaseReportRenderer", "renderer_sha256":flow.renderer_digest,
            "history":history.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
            "report_id":target, "estimated_total_ms":initial["estimated_total_ms"],
            "observed_total_ms":actual_ms, "queue":queue, "queued_predecessors":predecessors,
            "delivery_id":delivery_id, "captured_message":envelope,
            "initial":initial, "ready":ready, "after_delivery":accepted,
            "fresh_mfa":"recovery", "downloads_exact_after_reentry":true,
            "delivery_scope":"captured port acceptance, not an external inbox",
        })
    );
}
