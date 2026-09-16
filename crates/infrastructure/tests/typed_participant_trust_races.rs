mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod declaration_fixture;
mod typed_participant_service_support;
mod typed_participant_trust_races_support;

use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustStore},
    typed_participants::TypedParticipantWorkflow,
    ApplicationError,
};
use infrastructure::PostgresCredentialTrustStore;
use postgres::{Client, NoTls};
use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};
use typed_participant_service_support::FormatCheck;
use typed_participant_trust_races_support::*;

#[test]
fn crl_publication_during_support_validation_rejects_the_previously_captured_trust() {
    let Some(mut s) = Scenario::new() else { return };
    let publisher =
        Arc::new(PostgresCredentialTrustStore::open(&s.db.admin_url, s.clock.clone()).unwrap());
    let successor = inspection(2, -50);
    let expected = s.trust.revision;
    let verifier = Arc::new(CapturingVerifier::default());
    let workflow = s.workflow(
        FormatCheck(Some(Box::new(move || {
            publisher
                .publish(
                    CredentialTrustExpectation::Revision(expected),
                    successor.clone(),
                )
                .unwrap();
        }))),
        verifier.clone(),
        "typed-trust-publication",
    );
    let before = audit(&mut s.db);
    let result = workflow.submit_participant("session", s.db.case, s.submission());
    assert!(
        matches!(result, Err(ApplicationError::CredentialTrustChanged)),
        "{result:?}"
    );
    let captured = verifier.0.lock().unwrap().clone().unwrap();
    assert_eq!(captured.trust, s.trust.inspection);
    assert_eq!(captured.checked_at, s.db.at.unix_timestamp());
    no_participant_changes(&mut s.db);
    let after = audit(&mut s.db);
    assert_eq!(&after[..before.len()], before.as_slice());
    assert_eq!(
        after[before.len()..]
            .iter()
            .map(|a| a["action"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "participant.prepared",
            "participant.credential_trust_published"
        ]
    );
    assert_eq!(
        s.db.admin
            .query_one(
                "SELECT count(*) FROM participant_credential_trust_revisions",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}

#[test]
fn expiration_while_commit_waits_for_the_audit_lock_rejects_without_rewriting_check_time() {
    let Some(mut s) = Scenario::new() else { return };
    let locker = Arc::new(Mutex::new(Client::connect(&s.db.admin_url, NoTls).unwrap()));
    let lock_in_parser = locker.clone();
    let verifier = Arc::new(CapturingVerifier::default());
    let name = format!("typed-expiry-{}", uuid::Uuid::new_v4().simple());
    let workflow = s.workflow(
        FormatCheck(Some(Box::new(move || {
            lock_in_parser
                .lock()
                .unwrap()
                .batch_execute("BEGIN; SELECT pg_advisory_xact_lock(280603412820)")
                .unwrap();
        }))),
        verifier.clone(),
        &name,
    );
    let before = audit(&mut s.db);
    let case = s.db.case;
    let submission = s.submission();
    let worker =
        std::thread::spawn(move || workflow.submit_participant("session", case, submission));
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut waiting = false;
    while Instant::now() < deadline {
        if verifier.0.lock().unwrap().is_some() {
            waiting=s.db.control.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=$1 AND wait_event='advisory')",&[&name]).unwrap().get(0);
        }
        if waiting {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let captured = verifier.0.lock().unwrap().clone();
    s.clock
        .0
        .store(s.trust.inspection.valid_until + 1, Ordering::SeqCst);
    locker.lock().unwrap().batch_execute("COMMIT").unwrap();
    let result = worker.join().unwrap();
    assert!(
        waiting,
        "commit must have waited on the audit lock after successful verification"
    );
    let captured = captured.expect("real signature must have been verified before commit wait");
    assert_eq!(captured.checked_at, s.db.at.unix_timestamp());
    assert_eq!(captured.trust, s.trust.inspection);
    assert!(
        matches!(result, Err(ApplicationError::CredentialTrustChanged)),
        "{result:?}"
    );
    assert_eq!(
        verifier.0.lock().unwrap().as_ref().unwrap().checked_at,
        captured.checked_at
    );
    no_participant_changes(&mut s.db);
    let after = audit(&mut s.db);
    assert_eq!(&after[..before.len()], before.as_slice());
    assert_eq!(
        after[before.len()..]
            .iter()
            .map(|a| a["action"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["participant.prepared"]
    );
}
