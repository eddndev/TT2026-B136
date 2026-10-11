use crate::{
    serve_alert_supervisor::supervise_with_report_email, serve_deadline_runtime::RuntimeControl,
    serve_report_email_config::ReportEmailSettings, serve_report_email_runtime, serve_stop::Stop,
};
use application::{
    case_reports::{CaseReportEmailRun, CaseReportEmailState},
    ApplicationError,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

fn lookup(name: &str) -> Result<Option<String>, ApplicationError> {
    Ok(Some(
        match name {
            "TT_REPORT_EMAIL_ENABLED" => "true",
            "REPORT_EMAIL_FROM" => "reports@example.test",
            "REPORT_PUBLIC_URL" => "https://qadra.example.test/",
            "RESEND_API_KEY" => "test-report-key",
            _ => panic!("unexpected setting name"),
        }
        .into(),
    ))
}

#[test]
fn ready_report_email_is_disabled_by_default_without_reading_channel_secrets() {
    for enabled in [None, Some("false")] {
        let mut reads = vec![];
        let value = ReportEmailSettings::load_with(|name| {
            reads.push(name.to_owned());
            assert_eq!(name, "TT_REPORT_EMAIL_ENABLED");
            Ok(enabled.map(str::to_owned))
        })
        .unwrap();
        assert!(value.is_none());
        assert_eq!(reads, ["TT_REPORT_EMAIL_ENABLED"]);
    }
}

#[test]
fn enabled_report_email_requires_its_own_complete_sender_origin_and_secret() {
    for missing in ["REPORT_EMAIL_FROM", "REPORT_PUBLIC_URL", "RESEND_API_KEY"] {
        let value = ReportEmailSettings::load_with(|name| {
            if name == missing {
                Ok(None)
            } else {
                lookup(name)
            }
        });
        assert!(matches!(
            value,
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
    let settings = ReportEmailSettings::load_with(lookup).unwrap().unwrap();
    assert_eq!(settings.api_key.as_str(), "test-report-key");
    let template = settings.configuration.template();
    assert_eq!(template.from, "reports@example.test");
    assert!(template.text.contains("https://qadra.example.test/"));
}

#[test]
fn report_email_rejects_ambiguous_activation_and_invalid_secrets_without_echoing_them() {
    for flag in ["", "1", "yes", "TRUE", " true "] {
        let value = ReportEmailSettings::load_with(|name| {
            assert_eq!(name, "TT_REPORT_EMAIL_ENABLED");
            Ok(Some(flag.into()))
        });
        assert!(matches!(
            value,
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
    for secret in [
        String::new(),
        "private key marker\n".into(),
        "k".repeat(1025),
    ] {
        let value = ReportEmailSettings::load_with(|name| {
            if name == "RESEND_API_KEY" {
                Ok(Some(secret.clone()))
            } else {
                lookup(name)
            }
        });
        let error = match value {
            Err(error) => error,
            Ok(_) => panic!("invalid secret accepted"),
        };
        assert!(matches!(error, ApplicationError::InvalidConfiguration(_)));
        assert!(!error.to_string().contains("private key marker"));
    }
}

struct Control {
    remaining: AtomicUsize,
    waits: AtomicUsize,
}
impl Control {
    fn new(remaining: usize) -> Self {
        Self {
            remaining: AtomicUsize::new(remaining),
            waits: AtomicUsize::new(0),
        }
    }
}
impl RuntimeControl for Control {
    fn is_stopped(&self) -> bool {
        self.remaining.load(SeqCst) == 0
    }
    fn wait(&self, duration: Duration) {
        assert_eq!(duration, Duration::from_secs(1));
        self.remaining.fetch_sub(1, SeqCst);
        self.waits.fetch_add(1, SeqCst);
    }
}

#[test]
fn report_email_runtime_never_claims_after_stop() {
    serve_report_email_runtime::run(&|| panic!("must not claim after stop"), &Control::new(0))
        .unwrap();
}

#[test]
fn report_email_runtime_serially_pauses_after_every_outcome_and_transient_failure() {
    let values = RefCell::new(VecDeque::from([
        Ok(CaseReportEmailRun::Idle),
        Ok(CaseReportEmailRun::Stale),
        Ok(CaseReportEmailRun::Recorded(CaseReportEmailState::Accepted)),
        Ok(CaseReportEmailRun::Recorded(
            CaseReportEmailState::Uncertain,
        )),
        Err(ApplicationError::Port("private adapter failure".into())),
    ]));
    let control = Control::new(5);
    serve_report_email_runtime::run(&|| values.borrow_mut().pop_front().unwrap(), &control)
        .unwrap();
    assert!(values.borrow().is_empty());
    assert_eq!(control.waits.load(SeqCst), 5);
}

#[test]
fn report_email_runtime_propagates_permanent_failure_for_supervision() {
    let control = Control::new(1);
    let value = serve_report_email_runtime::run(
        &|| {
            Err(ApplicationError::InvalidConfiguration(
                "private configuration".into(),
            ))
        },
        &control,
    );
    assert!(matches!(
        value,
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert_eq!(control.waits.load(SeqCst), 0);
}

fn wait_for_stop(
    stop: Arc<Stop>,
    done: Arc<AtomicUsize>,
) -> tokio::task::JoinHandle<Result<(), ApplicationError>> {
    tokio::spawn(async move {
        stop.stopped().await;
        done.fetch_add(1, SeqCst);
        Ok(())
    })
}

#[tokio::test]
async fn report_email_exit_stops_and_joins_all_existing_consumers() {
    let stop = Arc::new(Stop::default());
    let done = Arc::new(AtomicUsize::new(0));
    let worker = || wait_for_stop(stop.clone(), done.clone());
    let email = tokio::spawn(async {
        Err(ApplicationError::InvalidConfiguration(
            "private provider detail".into(),
        ))
    });
    let value = tokio::time::timeout(
        Duration::from_secs(3),
        supervise_with_report_email(
            worker(),
            worker(),
            worker(),
            Some(worker()),
            Some(worker()),
            Some(email),
            stop.clone(),
        ),
    )
    .await
    .unwrap();
    assert!(stop.is_stopped());
    assert_eq!(done.load(SeqCst), 5);
    let error = value.unwrap_err().to_string();
    assert!(error.contains("report_email_failed"));
    assert!(!error.contains("private provider"));
}

#[tokio::test]
async fn existing_consumer_exit_stops_and_joins_optional_report_email() {
    let stop = Arc::new(Stop::default());
    let done = Arc::new(AtomicUsize::new(0));
    let worker = || wait_for_stop(stop.clone(), done.clone());
    let deadlines =
        tokio::spawn(async { Err(ApplicationError::Port("private database detail".into())) });
    let value = tokio::time::timeout(
        Duration::from_secs(3),
        supervise_with_report_email(
            deadlines,
            worker(),
            worker(),
            Some(worker()),
            Some(worker()),
            Some(worker()),
            stop.clone(),
        ),
    )
    .await
    .unwrap();
    assert!(stop.is_stopped());
    assert_eq!(done.load(SeqCst), 5);
    let error = value.unwrap_err().to_string();
    assert!(error.contains("deadline_failed"));
    assert!(!error.contains("private database"));
}

#[tokio::test]
async fn requested_stop_joins_all_owners_with_report_email_enabled_or_disabled() {
    for enabled in [false, true] {
        let stop = Arc::new(Stop::default());
        stop.request();
        let done = Arc::new(AtomicUsize::new(0));
        let worker = || wait_for_stop(stop.clone(), done.clone());
        tokio::time::timeout(
            Duration::from_secs(3),
            supervise_with_report_email(
                worker(),
                worker(),
                worker(),
                Some(worker()),
                Some(worker()),
                enabled.then(worker),
                stop.clone(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(done.load(SeqCst), if enabled { 6 } else { 5 });
    }
}
