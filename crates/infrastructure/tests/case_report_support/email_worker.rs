use super::*;
use application::ApplicationError;
use std::{collections::VecDeque, sync::Mutex};

#[derive(Default)]
struct Capture {
    attempts: Mutex<Vec<(Uuid, CaseReportEmailMessage)>>,
    outcomes: Mutex<VecDeque<CaseReportEmailOutcome>>,
}

impl Capture {
    fn outcomes(values: impl IntoIterator<Item = CaseReportEmailOutcome>) -> Self {
        Self {
            attempts: Mutex::new(Vec::new()),
            outcomes: Mutex::new(values.into_iter().collect()),
        }
    }

    fn attempts(&self) -> Vec<(Uuid, CaseReportEmailMessage)> {
        self.attempts.lock().unwrap().clone()
    }
}

impl CaseReportEmailDelivery for Capture {
    fn deliver(
        &self,
        id: Uuid,
        message: &CaseReportEmailMessage,
    ) -> Result<CaseReportEmailOutcome, ApplicationError> {
        self.attempts.lock().unwrap().push((id, message.clone()));
        Ok(self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider attempt"))
    }
}

#[test]
fn worker_reopens_real_store_and_retries_the_same_captured_delivery() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = Arc::new(store(&db, timer.clone()));
    let ready = ready(&mut db, &initial, &actor);
    let capture = Arc::new(Capture::outcomes([
        CaseReportEmailOutcome::Uncertain,
        CaseReportEmailOutcome::Accepted,
    ]));
    let worker =
        CaseReportEmailWorker::new(initial.clone(), capture.clone(), template(), timer.clone());
    assert_eq!(
        worker.run_once().unwrap(),
        CaseReportEmailRun::Recorded(CaseReportEmailState::Uncertain)
    );
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Idle);
    let first = capture.attempts();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].1.to, actor.email);
    assert_eq!(first[0].1.from, template().from);
    let saved = delivery(&mut db, ready.id).unwrap();
    assert_eq!(saved["state"], "uncertain");
    assert_eq!(
        saved["delivery_id"].as_str().unwrap(),
        first[0].0.to_string()
    );
    let retry_at = OffsetDateTime::parse(saved["retry_at"].as_str().unwrap(), &Rfc3339).unwrap();
    assert_eq!(retry_at, at + Duration::seconds(60));
    drop(worker);
    drop(initial);

    timer.set(retry_at);
    let reopened = Arc::new(store(&db, timer.clone()));
    let changed = CaseReportEmailTemplate {
        from: "different@example.test".into(),
        subject: "Changed configuration after restart".into(),
        text: "https://different.example.test/#reports".into(),
    };
    let worker = CaseReportEmailWorker::new(reopened.clone(), capture.clone(), changed, timer);
    assert_eq!(
        worker.run_once().unwrap(),
        CaseReportEmailRun::Recorded(CaseReportEmailState::Accepted)
    );
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Idle);
    let attempts = capture.attempts();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0], attempts[1]);
    let accepted = delivery(&mut db, ready.id).unwrap();
    for field in ["delivery_id", "message", "provider_deadline"] {
        assert_eq!(accepted[field], saved[field]);
    }
    assert_eq!(accepted["state"], "accepted");
    assert!(accepted["retry_at"].is_null());
    let detail = reopened.get(&actor, ready.id, retry_at).unwrap();
    assert_eq!(detail.state, ready.state);
    assert_eq!(detail.ready_email, Some(CaseReportEmailState::Accepted));
    assert_eq!(audits(&mut db, "case_report.email_claim"), 2);
    assert_eq!(audits(&mut db, "case_report.email_finish"), 2);
}

#[test]
fn worker_checks_real_membership_and_never_calls_delivery_after_revocation() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    db.case = seed_case(&mut db, "Captured report membership", at);
    let id = db.user("litigator", true);
    let actor = principal(&mut db, id);
    let timer = clock(at);
    let store = Arc::new(store(&db, timer.clone()));
    let ready = ready(&mut db, &store, &actor);
    let capture = Arc::new(Capture::default());
    let worker = CaseReportEmailWorker::new(store, capture.clone(), template(), timer);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &id.as_uuid()],
        )
        .unwrap();
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Idle);
    assert_eq!(worker.run_once().unwrap(), CaseReportEmailRun::Idle);
    assert!(capture.attempts().is_empty());
    assert_eq!(
        delivery(&mut db, ready.id).unwrap()["state"],
        "access_revoked"
    );
    assert_eq!(audits(&mut db, "case_report.email_revoke"), 1);
    assert_eq!(audits(&mut db, "case_report.email_claim"), 0);
}
