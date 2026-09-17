mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;
use application::{cases::CurrentCaseAdministration, procedural_facts::*, ApplicationError};
use domain::identity::Role;
use infrastructure::RingSha256Hasher;
use procedural_fact_backend_support::*;

#[test]
fn resolution_prepare_reserves_nothing_and_commit_captures_real_r0_without_profile() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let command = record();
    let before = snapshot(&mut db);
    let prepared = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    assert_eq!(snapshot(&mut db), before);
    let CurrentCaseAdministration::Unrevised(metadata) = &prepared.observed_administration else {
        panic!("R0 expected")
    };
    assert_eq!(
        (metadata.title(), metadata.reference()),
        ("Baseline", "REF-OLD")
    );
    assert!(prepared.sources.resolved.resolution.is_none());
    assert!(prepared.sources.direct_supports.is_empty());
    let saved = workflow
        .submit(
            "session",
            db.case,
            command.clone(),
            prepared.submission_digest,
        )
        .unwrap();
    assert_eq!(saved.snapshot.metadata().revision.get(), 1);
    assert_eq!(
        saved.snapshot.metadata().recorded_administration,
        prepared.observed_administration
    );
    assert_eq!(saved.snapshot.metadata().recorded_by.id, db.owner);
    assert_eq!(
        saved.snapshot.metadata().recorded_by.email,
        "owner@example.test"
    );
    assert_eq!(saved.snapshot.metadata().recorded_at, db.at);
    assert_eq!(
        saved.snapshot.metadata().receipt.operation_id,
        command.operation_id()
    );
    assert_eq!(
        saved.snapshot.metadata().receipt.submission_digest,
        prepared.submission_digest
    );
    assert_eq!(counts(&mut db), (1, 1, 1));
    fact_receipt_matches(&RingSha256Hasher, &saved).unwrap();
}

#[test]
fn both_families_preserve_exact_revisions_through_correction_and_terminal_withdrawal() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let resolution = persist(&workflow, db.case, record());
    let notification = persist(&workflow, db.case, notify(resolution_ref(&resolution)));
    for first in [resolution, notification] {
        let before = counts(&mut db);
        let second = persist(&workflow, db.case, correct(&first));
        let third = persist(&workflow, db.case, withdraw(&second));
        assert_eq!(third.snapshot.metadata().revision.get(), 3);
        assert_eq!(third.snapshot.metadata().status, FactStatus::Withdrawn);
        assert_eq!(third.snapshot.values(), second.snapshot.values());
        assert_eq!(third.sources, second.sources);
        assert_eq!(third.snapshot.target(), first.snapshot.target());
        assert_eq!(counts(&mut db), (before.0, before.1 + 2, before.2 + 2));
        assert_eq!(
            workflow
                .get(
                    "session",
                    db.case,
                    first.snapshot.target(),
                    Some(FactRevision::initial())
                )
                .unwrap(),
            first
        );
        assert_eq!(
            workflow
                .get("session", db.case, first.snapshot.target(), None)
                .unwrap(),
            third
        );
        let history = workflow
            .history(
                "session",
                db.case,
                first.snapshot.target(),
                FactHistoryQuery::new(20, None).unwrap(),
            )
            .unwrap();
        assert_eq!(
            history.revisions,
            vec![
                FactHistoryEntry::from(&third.snapshot),
                FactHistoryEntry::from(&second.snapshot),
                FactHistoryEntry::from(&first.snapshot),
            ]
        );
        assert!(!history.has_more);
        let before = snapshot(&mut db);
        assert!(matches!(
            workflow.prepare("session", db.case, correct(&third)),
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::AlreadyWithdrawn
            ))
        ));
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn notification_can_select_an_exact_historical_parent_after_its_withdrawal() {
    let Some(db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, db.case, record());
    let second = persist(&workflow, db.case, correct(&first));
    let third = persist(&workflow, db.case, withdraw(&second));
    for source in [&first, &third] {
        let selected = resolution_ref(source);
        let saved = persist(&workflow, db.case, notify(selected));
        let captured = saved.sources.resolved.resolution.unwrap();
        assert_eq!(captured.reference, selected);
        assert_eq!(captured.status, source.snapshot.metadata().status);
        assert_eq!(
            captured.values_digest,
            source.snapshot.metadata().values_digest
        );
        assert_eq!(
            captured.submission_digest,
            source.snapshot.metadata().receipt.submission_digest
        );
        assert_eq!(
            saved.sources.views.resolution.as_ref().unwrap().reference,
            selected
        );
        let FactTarget::Notification { resolution_id, .. } = saved.snapshot.target() else {
            unreachable!()
        };
        assert_eq!(resolution_id, selected.id);
        fact_receipt_matches(&RingSha256Hasher, &saved).unwrap();
    }
    let listed = workflow
        .list_notifications(
            "session",
            db.case,
            resolution_ref(&first).id,
            NotificationQuery::new(20, None, FactStatusFilter::Recorded).unwrap(),
        )
        .unwrap();
    assert_eq!(listed.notifications.len(), 2);
    assert!(!listed.has_more);
}
