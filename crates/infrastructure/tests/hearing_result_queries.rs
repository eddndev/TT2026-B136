mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;

use application::{hearing_results::*, ApplicationError};
use domain::{hearings::HearingId, identity::Role};
use hearing_result_database_support::*;
use uuid::Uuid;

fn seed(db: &mut Fixture) -> HearingId {
    hearing_database_support::complete(db);
    hearing_database_support::persist(
        &hearing_database_support::service(db, db.owner, Role::Owner),
        db.case,
        hearing_database_support::schedule(),
    )
    .snapshot
    .id
}
fn correct(first: &HearingResultDetail) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: first.snapshot.hearing_id,
        result_id: first.snapshot.id,
        change: HearingResultChange::Correct {
            expected_revision: first.snapshot.revision,
            values: values("Corrected declaration"),
            reason: HearingResultText::new("Correct transcription").unwrap(),
        },
    }
}

#[test]
fn list_filters_heads_before_limit_and_preserves_the_nil_uuid() {
    let Some(mut db) = Fixture::new() else { return };
    let hearing = seed(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    let mut roots = Vec::new();
    for id in 0..4 {
        let mut command = record(hearing);
        command.result_id = HearingResultId::from_uuid(Uuid::from_u128(id));
        roots.push(persist(&svc, db.case, command));
    }
    let updated = persist(&svc, db.case, correct(&roots[0]));
    persist(
        &svc,
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: hearing,
            result_id: roots[1].snapshot.id,
            change: HearingResultChange::Withdraw {
                expected_revision: roots[1].snapshot.revision,
                reason: HearingResultText::new("Withdraw declaration").unwrap(),
            },
        },
    );
    let mut cursor = None;
    let mut ids = Vec::new();
    loop {
        let page = svc
            .list(
                "session",
                db.case,
                hearing,
                HearingResultQuery::new(1, cursor, HearingResultStatusFilter::Recorded).unwrap(),
            )
            .unwrap();
        assert_eq!(page.results.len(), 1);
        if ids.is_empty() {
            assert_eq!(page.results[0].id, updated.snapshot.id);
            assert_eq!(page.results[0].revision.get(), 2);
        }
        ids.extend(page.results.iter().map(|r| r.id));
        if !page.has_more {
            assert!(page.next_after_id.is_none());
            break;
        }
        cursor = page.next_after_id;
    }
    assert_eq!(
        ids,
        [0, 2, 3].map(|id| HearingResultId::from_uuid(Uuid::from_u128(id)))
    );
    let withdrawn = svc
        .list(
            "session",
            db.case,
            hearing,
            HearingResultQuery::new(1, None, HearingResultStatusFilter::Withdrawn).unwrap(),
        )
        .unwrap();
    assert_eq!(withdrawn.results[0].id, roots[1].snapshot.id);
    assert_eq!(withdrawn.results[0].revision.get(), 2);
    assert!(!withdrawn.has_more);
    let all = svc
        .list(
            "session",
            db.case,
            hearing,
            HearingResultQuery::new(100, None, HearingResultStatusFilter::All).unwrap(),
        )
        .unwrap();
    assert_eq!(all.results.len(), 4);
}

#[test]
fn history_limits_twenty_lightweight_exact_revisions_and_excludes_the_cursor() {
    let Some(mut db) = Fixture::new() else { return };
    let hearing = seed(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    let mut current = persist(&svc, db.case, record(hearing));
    let first = current.clone();
    for _ in 1..22 {
        current = persist(&svc, db.case, correct(&current));
    }
    let page = svc
        .history(
            "session",
            db.case,
            hearing,
            first.snapshot.id,
            HearingResultHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(page.revisions.len(), 20);
    assert!(page.has_more);
    assert_eq!(page.revisions[0].revision.get(), 22);
    assert_eq!(page.revisions[19].revision.get(), 3);
    for revision in &page.revisions {
        hearing_result_history_receipt_matches(&infrastructure::RingSha256Hasher, revision)
            .unwrap();
    }
    let tail = svc
        .history(
            "session",
            db.case,
            hearing,
            first.snapshot.id,
            HearingResultHistoryQuery::new(20, page.next_before_revision.map(|r| r.get())).unwrap(),
        )
        .unwrap();
    assert_eq!(
        tail.revisions
            .iter()
            .map(|r| r.revision.get())
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert!(!tail.has_more);
    assert!(tail.next_before_revision.is_none());
    assert_eq!(
        tail.revisions[1],
        HearingResultHistoryEntry::from(&first.snapshot)
    );
}

#[test]
fn queries_reauthorize_membership_role_and_scope_without_auditing_rejections() {
    let Some(mut db) = Fixture::new() else { return };
    let hearing = seed(&mut db);
    let owner = service(&db, db.owner, Role::Owner);
    let first = persist(&owner, db.case, record(hearing));
    let member = db.user("litigator", true);
    let outsider = db.user("litigator", false);
    let member_svc = service(&db, member, Role::Litigator);
    let outsider_svc = service(&db, outsider, Role::Litigator);
    assert_eq!(
        member_svc
            .get("session", db.case, hearing, first.snapshot.id, None)
            .unwrap(),
        first
    );
    let before = counts(&mut db);
    assert!(matches!(
        outsider_svc.get("session", db.case, hearing, first.snapshot.id, None),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        owner.list(
            "session",
            db.case,
            HearingId::new(),
            HearingResultQuery::new(1, None, HearingResultStatusFilter::All).unwrap()
        ),
        Err(ApplicationError::HearingResult(
            HearingResultError::ReferenceNotFound
        ))
    ));
    assert!(matches!(
        owner.get(
            "session",
            db.case,
            HearingId::new(),
            first.snapshot.id,
            None
        ),
        Err(ApplicationError::HearingResult(
            HearingResultError::NotFound
        ))
    ));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&member.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        member_svc.history(
            "session",
            db.case,
            hearing,
            first.snapshot.id,
            HearingResultHistoryQuery::new(20, None).unwrap()
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    db.admin
        .execute(
            "UPDATE users SET role='client' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        owner.list(
            "session",
            db.case,
            hearing,
            HearingResultQuery::new(1, None, HearingResultStatusFilter::All).unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(counts(&mut db), before);
}
