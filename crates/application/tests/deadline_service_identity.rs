#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;
use application::{
    deadline_reevaluation::PredecessorReceipt,
    deadline_tracking::{DeadlineReviewState, TrackingPolicies, TrackingPolicy},
    deadlines::*,
    identity::Principal,
    ApplicationError,
};
use deadline_service_support::*;
use domain::identity::Role;
use mockall::Sequence;

fn policies() -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Follow,
        source: TrackingPolicy::Follow,
        calendar: TrackingPolicy::Undetermined,
    }
}

fn person(email: &str) -> Principal {
    let mut value = principal(Role::Owner);
    value.email = email.into();
    value
}

fn identity_emails(emails: &[&str]) -> MockIdentity {
    let mut identity = MockIdentity::new();
    let mut sequence = Sequence::new();
    for email in emails {
        let value = person(email);
        identity
            .expect_authenticate()
            .withf(|token| token == "session")
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(value));
    }
    identity
}

fn expect_preparation(store: &mut MockStore, preparation: DeadlinePreparation, count: usize) {
    store
        .expect_prepare()
        .times(count)
        .returning(move |actor, case, _| {
            assert_eq!(actor, owner());
            assert_eq!(case, case_id());
            Ok(preparation.clone())
        });
}

fn human(command: DeadlineCommand, declared: TrackingPolicies) -> DeadlineHumanCommand {
    DeadlineHumanCommand::new(command, Some(declared)).unwrap()
}

#[test]
fn changing_only_the_authenticated_email_changes_the_confirmation_digest() {
    let (command, preparation) = fixture();
    let mut drafts = Vec::new();
    for email in ["first@example.test", "second@example.test"] {
        let mut store = MockStore::new();
        expect_preparation(&mut store, preparation.clone(), 1);
        store.expect_commit().times(0);
        let (workflow, _) = service(store, identity_emails(&[email, email]));
        drafts.push(
            workflow
                .prepare("session", case_id(), human(command.clone(), policies()))
                .unwrap(),
        );
    }
    assert_eq!(drafts[0].command, drafts[1].command);
    assert_eq!(drafts[0].calculation, drafts[1].calculation);
    assert_eq!(drafts[0].responsible, drafts[1].responsible);
    assert_ne!(drafts[0].submission_digest, drafts[1].submission_digest);
}

#[test]
fn changing_only_email_during_reauthentication_denies_prepare_and_submit() {
    for submitting in [false, true] {
        let (command, preparation) = fixture();
        let mut store = MockStore::new();
        expect_preparation(&mut store, preparation, if submitting { 2 } else { 1 });
        store.expect_commit().times(0);
        let emails = if submitting {
            vec![
                "before@example.test",
                "before@example.test",
                "before@example.test",
                "after@example.test",
            ]
        } else {
            vec!["before@example.test", "after@example.test"]
        };
        let (workflow, _) = service(store, identity_emails(&emails));
        let result = if submitting {
            let draft = workflow
                .prepare("session", case_id(), human(command.clone(), policies()))
                .unwrap();
            workflow
                .submit(
                    "session",
                    case_id(),
                    human(command, policies()),
                    draft.submission_digest,
                )
                .map(|_| ())
        } else {
            workflow
                .prepare("session", case_id(), human(command, policies()))
                .map(|_| ())
        };
        assert!(matches!(result, Err(ApplicationError::InvalidSession)));
    }
}

#[test]
fn changing_only_an_explicit_policy_requires_a_new_confirmation() {
    let (command, preparation) = fixture();
    let mut store = MockStore::new();
    expect_preparation(&mut store, preparation, 2);
    store.expect_commit().times(0);
    let (workflow, _) = service(store, identity(Role::Owner, 3));
    let draft = workflow
        .prepare("session", case_id(), human(command.clone(), policies()))
        .unwrap();
    let changed = TrackingPolicies {
        source: TrackingPolicy::Fixed,
        ..policies()
    };
    assert!(matches!(
        workflow.submit(
            "session",
            case_id(),
            human(command, changed),
            draft.submission_digest,
        ),
        Err(ApplicationError::Deadline(
            DeadlineError::SubmissionMismatch
        ))
    ));
}

#[test]
fn registration_and_correction_commit_the_authenticated_author_and_tracked_receipt() {
    for correction in [false, true] {
        let (mut command, mut preparation) = fixture();
        let predecessor = if correction {
            let base = captured();
            command.operation_id = DeadlineOperationId::new();
            command.change = DeadlineChange::Correct {
                expected_revision: base.revision,
                definition: base.definition.clone(),
                reason: evaluation::text("Explicitly review the recorded inputs"),
            };
            let predecessor = PredecessorReceipt {
                submission_digest: base.receipt.submission_digest,
                capture_digest: base.receipt.capture_digest,
            };
            preparation.base = Some(base);
            Some(predecessor)
        } else {
            None
        };
        let declared = TrackingPolicies {
            source: TrackingPolicy::Fixed,
            ..policies()
        };
        let wanted_author = DeadlineActorSnapshot::User {
            id: owner(),
            email: "authenticated@example.test".into(),
        };
        let expected_author = wanted_author.clone();
        let expected_command = command.clone();
        let mut store = MockStore::new();
        expect_preparation(&mut store, preparation, 2);
        store
            .expect_commit()
            .times(1)
            .return_once(move |actor, prepared| {
                assert_eq!(actor, owner());
                assert_eq!(prepared.command(), &expected_command);
                assert_eq!(prepared.tracked_author(), Some(&wanted_author));
                let tracking = prepared.tracking().unwrap();
                assert_eq!(tracking.policies, declared);
                assert_eq!(tracking.review.state(), DeadlineReviewState::Accepted);
                let receipt = prepared.receipt();
                let DeadlineReceiptVersion::Tracked(metadata) = &receipt.version else {
                    panic!("human writes must use tracked receipts");
                };
                assert_eq!(metadata.predecessor, predecessor);
                assert_eq!(metadata.cause, None);
                let mut returned = detail(&prepared);
                returned.tracking = prepared.tracking().cloned();
                returned.receipt = receipt;
                returned.recorded_by = wanted_author;
                deadline_receipt_matches(hasher().as_ref(), &returned).unwrap();
                Ok(returned)
            });
        let (workflow, _) = service(store, identity_emails(&["authenticated@example.test"; 4]));
        let draft = workflow
            .prepare("session", case_id(), human(command.clone(), declared))
            .unwrap();
        assert_eq!(draft.author, expected_author);
        assert_eq!(draft.tracking.policies, declared);
        assert_eq!(draft.tracking.review.state(), DeadlineReviewState::Accepted);
        let DeadlineReceiptVersion::Tracked(metadata) = &draft.receipt_version else {
            panic!("human drafts must expose tracked receipt metadata");
        };
        assert_eq!(metadata.predecessor, predecessor);
        assert_eq!(metadata.cause, None);
        let committed = workflow
            .submit(
                "session",
                case_id(),
                human(command, declared),
                draft.submission_digest,
            )
            .unwrap();
        assert_eq!(committed.recorded_by, expected_author);
        assert_eq!(committed.responsible.email, "owner@example.com");
        assert_eq!(committed.receipt.submission_digest, draft.submission_digest);
        assert_eq!(committed.tracking.unwrap().policies, declared);
    }
}
