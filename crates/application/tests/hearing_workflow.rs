#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_support;

use application::{hearings::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use hearing_support::*;
use std::sync::{atomic::Ordering, Arc};

#[test]
fn preparation_is_stateless_and_empty_supports_never_enter_crypto_parser() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let command = command();
    let preparation = preparation(case_id, actor.id);
    let mut store = MockStore::new();
    let expected = command.clone();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |id, case, input, limits| {
            assert_eq!(id, actor.id);
            assert_eq!(case, case_id);
            assert_eq!(input, &expected);
            assert_eq!(limits.max_documents(), 2);
            Ok(preparation)
        });
    let validator = Arc::new(Validator::default());
    let (service, clock) = service(store, identity, validator.clone());
    let draft = service
        .prepare("session", case_id, command.clone())
        .unwrap();
    assert_eq!(draft.command, command);
    assert_eq!(draft.actor, actor.id);
    assert_eq!(draft.result_revision, HearingRevision::initial());
    assert_eq!(draft.values, values());
    assert_eq!(
        draft.values_digest,
        hearing_values_digest(hasher().as_ref(), &draft.values)
    );
    assert_eq!(
        draft.submission_digest,
        hearing_submission_digest(
            hasher().as_ref(),
            actor.id,
            case_id,
            &command,
            draft.values_digest
        )
    );
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    assert_eq!(clock.calls(), 0);
}

#[test]
fn submission_reprepares_and_commits_the_exact_receipt_without_sampling_service_clock() {
    let (identity, actor) = identity(Role::Litigator, 2);
    let case_id = CaseId::new();
    let command = command();
    let prep = preparation(case_id, actor.id);
    let expected = detail(case_id, actor.id, &command, values(), &prep.context);
    let result = expected.clone();
    let digest = expected.snapshot.receipt.submission_digest;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let submitted = command.clone();
    store
        .expect_commit()
        .times(1)
        .return_once(move |id, case, prepared| {
            assert_eq!(id, actor.id);
            assert_eq!(case, case_id);
            assert_eq!(prepared.command(), &submitted);
            assert_eq!(prepared.submission_digest(), digest);
            assert_eq!(prepared.values(), &values());
            assert_eq!(
                prepared.values_digest(),
                hearing_values_digest(hasher().as_ref(), &values())
            );
            assert_eq!(prepared.scheduling_context().stage_digest, None);
            assert!(prepared.formats().is_empty());
            Ok(result)
        });
    let (service, clock) = service(store, identity, Arc::new(Validator::default()));
    assert_eq!(
        service.submit("session", case_id, command, digest).unwrap(),
        expected
    );
    assert_eq!(clock.calls(), 0);
}

#[test]
fn no_op_replacement_is_a_new_revision_and_not_an_automatic_replay() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let base = detail(case_id, actor.id, &command(), values(), &prep.context);
    let command = replacement(&base);
    prep.base = Some(base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    let draft = service.prepare("session", case_id, command).unwrap();
    assert_eq!(draft.result_revision.get(), 2);
    assert_eq!(draft.values, values());
}

#[test]
fn client_cannot_read_and_paralegal_cannot_prepare_or_submit() {
    let (identity, _) = identity(Role::Client, 5);
    let case_id = CaseId::new();
    let id = HearingId::new();
    let (reader, clock) = service(MockStore::new(), identity, Arc::new(Validator::default()));
    let result = reader.context("session", case_id);
    assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
    assert!(matches!(
        reader.list(
            "session",
            case_id,
            HearingQuery::new(20, None, HearingStatusFilter::All).unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        reader.get("session", case_id, id, None),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        reader.history(
            "session",
            case_id,
            id,
            HearingHistoryQuery::new(20, None).unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        reader.agenda(
            "session",
            HearingAgendaQuery::new(
                20,
                instant(),
                instant() + time::Duration::days(1),
                None,
                HearingStatusFilter::All
            )
            .unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 0);
    let (identity, _) = hearing_support::identity(Role::Paralegal, 2);
    let (writer, clock) = service(MockStore::new(), identity, Arc::new(Validator::default()));
    assert!(matches!(
        writer.prepare("session", case_id, command()),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        writer.submit(
            "session",
            case_id,
            command(),
            domain::crypto::Sha256Digest::from_array([0; 32])
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn authorized_reads_preserve_queries_and_audit_times() {
    let (identity, actor) = identity(Role::Paralegal, 5);
    let case_id = CaseId::new();
    let cmd = command();
    let ctx = context(case_id, actor.id);
    let result = detail(case_id, actor.id, &cmd, values(), &ctx);
    let query = HearingQuery::new(4, None, HearingStatusFilter::All).unwrap();
    let history = HearingHistoryQuery::new(5, Some(9)).unwrap();
    let agenda = HearingAgendaQuery::new(
        6,
        instant(),
        instant() + time::Duration::days(2),
        None,
        HearingStatusFilter::Scheduled,
    )
    .unwrap();
    let mut store = MockStore::new();
    let expected = ctx.clone();
    store
        .expect_context()
        .times(1)
        .return_once(move |id, case, at| {
            assert_eq!((id, case, at), (actor.id, case_id, instant()));
            Ok(expected)
        });
    store
        .expect_list()
        .times(1)
        .return_once(move |id, case, q, at| {
            assert_eq!((id, case, q, at), (actor.id, case_id, query, instant()));
            Ok(HearingPage {
                hearings: vec![],
                has_more: false,
                next_after_id: None,
            })
        });
    let expected = result.clone();
    let hearing_id = cmd.hearing_id;
    store
        .expect_get()
        .times(1)
        .return_once(move |id, case, hearing, revision, at| {
            assert_eq!(
                (id, case, hearing, revision, at),
                (
                    actor.id,
                    case_id,
                    hearing_id,
                    Some(HearingRevision::initial()),
                    instant()
                )
            );
            Ok(expected)
        });
    let expected = result.clone();
    store
        .expect_history()
        .times(1)
        .return_once(move |_, _, _, q, at| {
            assert_eq!((q, at), (history, instant()));
            Ok(HearingHistoryPage {
                revisions: vec![expected],
                has_more: false,
                next_before_revision: None,
            })
        });
    store
        .expect_agenda()
        .times(1)
        .return_once(move |id, q, at| {
            assert_eq!((id, q, at), (actor.id, agenda, instant()));
            Ok(HearingAgendaPage {
                hearings: vec![],
                has_more: false,
                next_after: None,
            })
        });
    let (service, clock) = service(store, identity, Arc::new(Validator::default()));
    assert_eq!(service.context("session", case_id).unwrap(), ctx);
    assert!(service
        .list("session", case_id, query)
        .unwrap()
        .hearings
        .is_empty());
    assert_eq!(
        service
            .get(
                "session",
                case_id,
                cmd.hearing_id,
                Some(HearingRevision::initial())
            )
            .unwrap(),
        result
    );
    assert_eq!(
        service
            .history("session", case_id, cmd.hearing_id, history)
            .unwrap()
            .revisions,
        vec![result]
    );
    assert!(service
        .agenda("session", agenda)
        .unwrap()
        .hearings
        .is_empty());
    assert_eq!(clock.calls(), 5);
}

#[test]
fn cancelling_after_an_administrative_edit_copies_scheduling_context_and_historical_values() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let base = detail(case_id, actor.id, &command(), values(), &prep.context);
    let historical = base.snapshot.scheduling_context;
    if let application::cases::CurrentCaseAdministration::Recorded(ref mut current) =
        prep.context.administration
    {
        current.revision = domain::case_administration::CaseRevision::new(2).unwrap();
    }
    let cmd = cancellation(&base);
    let mut result = detail(
        case_id,
        actor.id,
        &cmd,
        base.snapshot.values.clone(),
        &prep.context,
    );
    result.snapshot.scheduling_context = historical;
    let digest = result.snapshot.receipt.submission_digest;
    let expected = result.clone();
    prep.base = Some(base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            assert_eq!(prepared.scheduling_context(), historical);
            assert_eq!(
                prepared
                    .preparation()
                    .context
                    .administration
                    .revision()
                    .unwrap()
                    .get(),
                2
            );
            assert!(prepared.formats().is_empty());
            Ok(result)
        });
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert_eq!(
        service.submit("session", case_id, cmd, digest).unwrap(),
        expected
    );
}
