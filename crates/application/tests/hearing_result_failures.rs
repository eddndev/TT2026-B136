#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_result_support;
#[allow(dead_code)]
mod hearing_support;

use application::{
    cases::{case_administration_digest, CurrentCaseAdministration},
    hearing_results::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::Role};
use hearing_result_support::*;
use std::sync::Arc;
#[test]
fn invalid_prepared_scope_administration_anchor_and_future_time_never_commit() {
    for mode in 0..8 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let mut cmd = command(&prep);
        match mode {
            0 => prep.case_id = CaseId::new(),
            1 => {
                if let CurrentCaseAdministration::Recorded(ref mut a) = prep.administration {
                    a.case_id = CaseId::new()
                }
            }
            2 => {
                if let CurrentCaseAdministration::Recorded(ref mut a) = prep.administration {
                    a.values_digest = Sha256Digest::from_array([99; 32])
                }
            }
            3 => {
                if let CurrentCaseAdministration::Recorded(ref mut a) = prep.administration {
                    a.values = a
                        .values
                        .with_status(domain::case_administration::CaseAdministrativeStatus::Closed);
                    a.values_digest = case_administration_digest(hasher().as_ref(), &a.values)
                }
            }
            4 => prep.anchor.snapshot.id = domain::hearings::HearingId::new(),
            5 => prep.anchor.snapshot.case_id = CaseId::new(),
            6 => {
                if let HearingResultChange::Record {
                    anchor_revision, ..
                } = &mut cmd.change
                {
                    *anchor_revision = domain::hearings::HearingRevision::new(2).unwrap()
                }
            }
            _ => {
                if let HearingResultChange::Record { values, .. } = &mut cmd.change {
                    let mut input = values_input(values);
                    input.event_time =
                        DeclaredHearingResultTime::instant(instant() + time::Duration::seconds(1))
                            .unwrap();
                    *values = HearingResultValues::new(input).unwrap()
                }
            }
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            service.prepare("session", case_id, cmd).is_err(),
            "mode {mode}"
        );
    }
}
#[test]
fn missing_stale_foreign_withdrawn_replayed_or_modified_base_is_rejected() {
    for mode in 0..8 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let mut base = detail(actor.id, &command(&prep), &prep);
        let mut cmd = correction(&base);
        match mode {
            0 => base.snapshot.case_id = CaseId::new(),
            1 => base.snapshot.id = HearingResultId::new(),
            2 => base.snapshot.revision = HearingResultRevision::new(3).unwrap(),
            3 => base.snapshot.status = HearingResultStatus::Withdrawn,
            4 => base.snapshot.values_digest = Sha256Digest::from_array([99; 32]),
            5 => cmd.operation_id = base.snapshot.receipt.operation_id,
            6 => base.snapshot.anchor.values_digest = Sha256Digest::from_array([99; 32]),
            _ => {}
        }
        prep.base = (mode != 7).then_some(base);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            service.prepare("session", case_id, cmd).is_err(),
            "mode {mode}"
        );
    }
}
#[test]
fn absent_foreign_self_or_substituted_exact_continuation_is_rejected() {
    for mode in 0..4 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let old = detail(actor.id, &command(&prep), &prep);
        let mut cmd = command(&prep);
        if let HearingResultChange::Record { continuation, .. } = &mut cmd.change {
            *continuation = Some(HearingResultContinuationRef::new(
                old.snapshot.id,
                old.snapshot.revision,
            ));
        }
        let mut previous = old.snapshot;
        match mode {
            0 => previous.case_id = CaseId::new(),
            1 => cmd.result_id = previous.id,
            2 => previous.revision = HearingResultRevision::new(2).unwrap(),
            _ => {}
        }
        prep.continuation = (mode != 3).then_some(previous);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            service.prepare("session", case_id, cmd).is_err(),
            "mode {mode}"
        );
    }
}
#[test]
fn mismatched_submission_or_expired_refreshed_session_never_commits() {
    for expires in [false, true] {
        let (mut identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let prep = preparation(case_id, actor.id);
        let cmd = command(&prep);
        let digest = if expires {
            detail(actor.id, &cmd, &prep)
                .snapshot
                .receipt
                .submission_digest
        } else {
            Sha256Digest::from_array([99; 32])
        };
        if expires {
            identity
                .expect_authenticate()
                .times(1)
                .return_once(|_| Err(ApplicationError::InvalidSession));
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        let result = service.submit("session", case_id, cmd, digest);
        if expires {
            assert!(matches!(result, Err(ApplicationError::InvalidSession)))
        } else {
            assert!(matches!(
                result,
                Err(ApplicationError::HearingResult(
                    HearingResultError::SubmissionMismatch
                ))
            ))
        }
    }
}

#[test]
fn observed_administration_cannot_predate_an_exact_continuation_source() {
    let (identity, actor) = identity(Role::Owner, 1);
    let case_id = CaseId::new();
    let mut earlier = preparation(case_id, actor.id);
    if let CurrentCaseAdministration::Recorded(ref mut admin) = earlier.administration {
        admin.revision = domain::case_administration::CaseRevision::new(2).unwrap();
    }
    let source = detail(actor.id, &command(&earlier), &earlier);
    let mut prep = preparation(case_id, actor.id);
    let mut cmd = command(&prep);
    if let HearingResultChange::Record { continuation, .. } = &mut cmd.change {
        *continuation = Some(HearingResultContinuationRef::new(
            source.snapshot.id,
            source.snapshot.revision,
        ));
    }
    prep.continuation = Some(source.snapshot);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.prepare("session", case_id, cmd),
        Err(ApplicationError::HearingResult(
            HearingResultError::StoredInconsistent(_)
        ))
    ));
}
