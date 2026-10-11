use crate::serve_invitation_config::{InvitationOptions, InvitationSettings};
use zeroize::Zeroizing;

fn options() -> InvitationOptions {
    InvitationOptions {
        enabled: true,
        from_email: Some("invitations@example.test".into()),
        public_url: Some("https://qadra.example.test/".into()),
    }
}
#[test]
fn invitations_require_independent_explicit_enablement_and_valid_delivery_configuration() {
    let mut disabled = options();
    disabled.enabled = false;
    assert!(
        InvitationSettings::resolve(disabled, Some(Zeroizing::new("existing-key".into())))
            .unwrap()
            .is_none()
    );
    assert!(InvitationSettings::resolve(options(), None).is_err());
    for field in [0, 1] {
        let mut invalid = options();
        if field == 0 {
            invalid.from_email = None;
        } else {
            invalid.public_url = None;
        }
        assert!(
            InvitationSettings::resolve(invalid, Some(Zeroizing::new("test-key".into()))).is_err()
        );
    }
    assert!(
        InvitationSettings::resolve(options(), Some(Zeroizing::new("test-key".into())))
            .unwrap()
            .is_some()
    );
}

use crate::{serve_deadline_runtime::RuntimeControl, serve_invitation_runtime, serve_stop::Stop};
use application::{identity::invitations::InvitationDeliveryRun, ApplicationError};
use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
    time::Duration,
};
struct Control(AtomicUsize);
impl RuntimeControl for Control {
    fn is_stopped(&self) -> bool {
        self.0.load(SeqCst) == 0
    }
    fn wait(&self, duration: Duration) {
        assert_eq!(duration, Duration::from_secs(1));
        self.0.fetch_sub(1, SeqCst);
    }
}
#[test]
fn invitation_runtime_pauses_after_each_outcome_and_stops_before_another_claim() {
    let values = RefCell::new(VecDeque::from([
        Ok(InvitationDeliveryRun::Idle),
        Ok(InvitationDeliveryRun::Stale),
        Err(ApplicationError::Port("private failure".into())),
    ]));
    serve_invitation_runtime::run(
        &|| values.borrow_mut().pop_front().unwrap(),
        &Control(AtomicUsize::new(3)),
    )
    .unwrap();
    assert!(values.borrow().is_empty());
    let stopped = Stop::default();
    stopped.request();
    serve_invitation_runtime::run(&|| panic!("must not claim after stop"), &stopped).unwrap();
}

#[tokio::test]
async fn invitation_exit_stops_and_joins_existing_consumers_without_private_errors() {
    use crate::serve_alert_supervisor::supervise_with_invitations;
    use std::sync::Arc;
    let stop = Arc::new(Stop::default());
    let done = Arc::new(AtomicUsize::new(0));
    let consumer = || {
        let stop = stop.clone();
        let done = done.clone();
        tokio::spawn(async move {
            stop.stopped().await;
            done.fetch_add(1, SeqCst);
            Ok(())
        })
    };
    let invites =
        tokio::spawn(async { Err(ApplicationError::Port("private provider message".into())) });
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        supervise_with_invitations(
            consumer(),
            consumer(),
            consumer(),
            Some(consumer()),
            Some(invites),
            stop.clone(),
        ),
    )
    .await
    .unwrap();
    assert!(stop.is_stopped());
    assert_eq!(done.load(SeqCst), 4);
    let message = result.unwrap_err().to_string();
    assert!(message.contains("invitations_failed"));
    assert!(!message.contains("private provider"));
}

#[test]
fn invitation_restoration_command_requires_the_same_explicit_target_and_audit_head() {
    use clap::Parser;
    let args = [
        "despacho-cli",
        "database",
        "invalidate-restored-invitations",
        "--operation-id",
        "10000000-0000-4000-8000-000000000001",
        "--expected-database",
        "restored",
        "--expected-schema",
        "public",
        "--expected-empty-audit",
    ];
    assert!(crate::cli::Cli::try_parse_from(args).is_ok());
    assert!(crate::cli::Cli::try_parse_from(&args[..args.len() - 1]).is_err());
}

#[test]
fn the_integrated_cli_keeps_invitation_and_recovery_options_distinct() {
    use clap::CommandFactory;
    crate::cli::Cli::command().debug_assert();
}
