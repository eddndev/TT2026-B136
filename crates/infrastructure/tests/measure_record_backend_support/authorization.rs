use super::*;
use application::ApplicationError;
use domain::identity::Role;

fn not_found(result: Result<MeasureRecordDetail, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::MeasureRecordRead(
            MeasureRecordReadError::NotFound
        ))
    ));
}

fn denied(
    db: &mut Fixture,
    storage: &PostgresMeasureDecisionStore,
    actor: &Principal,
    target: PrecautionaryMeasureRef,
) {
    let before = snapshot(db);
    assert!(MeasureRecordReadStore::get(storage, actor, db.case, target.id()).is_err());
    assert!(MeasureRecordReadStore::exact(storage, actor, db.case, target).is_err());
    assert!(MeasureRecordReadStore::list(
        storage,
        actor,
        db.case,
        MeasureRecordReadQuery::default()
    )
    .is_err());
    assert_eq!(snapshot(db), before);
}

#[test]
fn record_reads_require_live_full_principal_and_case_membership() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, _, marked) = chain(&mut db);
    let expected = administrative(&marked);
    let storage = store(&db);
    for role in ["owner", "litigator", "paralegal"] {
        let user = db.user(role, role != "owner");
        let actor = crate::measure_fixture::principal(&mut db, user);
        same_detail(
            &reads(&db, actor.clone())
                .get("session", db.case, expected.reference.id())
                .unwrap(),
            &expected,
        );
        if role != "owner" {
            db.admin
                .execute(
                    "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                    &[&db.case.as_uuid(), &user.as_uuid()],
                )
                .unwrap();
            denied(&mut db, &storage, &actor, expected.reference);
        }
    }
    for role in ["client", "litigator", "paralegal"] {
        let user = db.user(role, role == "client");
        let actor = crate::measure_fixture::principal(&mut db, user);
        denied(&mut db, &storage, &actor, expected.reference);
    }
    for stale in [
        Principal {
            email: "stale@example.test".into(),
            ..seed.actor.clone()
        },
        Principal {
            role: Role::Litigator,
            ..seed.actor.clone()
        },
    ] {
        denied(&mut db, &storage, &stale, expected.reference);
    }
    same_detail(
        &MeasureRecordReadStore::get(
            storage.as_ref(),
            &seed.actor,
            db.case,
            expected.reference.id(),
        )
        .unwrap(),
        &expected,
    );
}

#[test]
fn record_lookup_requires_exact_case_identity_revision_and_digest() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, _, marked) = chain(&mut db);
    let original_case = db.case;
    let expected = administrative(&marked);
    let _second = crate::measure_fixture::setup(&mut db);
    let service = reads(&db, seed.actor);
    let before = snapshot(&mut db);
    not_found(service.get("session", db.case, expected.reference.id()));
    not_found(service.exact("session", db.case, expected.reference));
    not_found(service.get("session", original_case, MeasureId::new()));
    for reference in [
        PrecautionaryMeasureRef::new(
            expected.reference.id(),
            MeasureRevision::new(99).unwrap(),
            expected.reference.digest(),
        ),
        PrecautionaryMeasureRef::new(
            expected.reference.id(),
            expected.reference.revision(),
            Sha256Digest::from_array([255; 32]),
        ),
    ] {
        not_found(service.exact("session", original_case, reference));
    }
    assert_eq!(snapshot(&mut db), before);
    let empty = service
        .list("session", db.case, MeasureRecordReadQuery::default())
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_id, None);
    same_detail(
        &service
            .exact("session", original_case, expected.reference)
            .unwrap(),
        &expected,
    );
}
