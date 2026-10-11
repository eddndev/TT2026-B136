use super::*;
use application::ApplicationError;

#[test]
fn ready_email_is_atomic_with_both_artifacts_notice_and_completion_audit() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Private report subject", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    let requested = estimated_request(&mut db, &store, &actor, Some(4000));
    assert!(delivery(&mut db, requested.id).is_none());
    let claimed = store.claim_next(at).unwrap().unwrap();
    let snapshot = store.capture(&claimed.lease, at).unwrap();
    let before = ledger(&mut db);
    reject_audit(
        &mut db,
        "case_report.complete",
        &format!(
        "EXISTS(SELECT 1 FROM case_report_email_deliveries WHERE report_id='{}' AND state='pending')
         AND (SELECT count(*) FROM case_report_artifacts WHERE report_id='{}')=2
         AND EXISTS(SELECT 1 FROM case_report_notices WHERE report_id='{}' AND kind='ready')",
        requested.id, requested.id, requested.id,
    ),
    );
    assert!(store
        .complete(&claimed.lease, &snapshot, artifacts(&snapshot), at)
        .is_err());
    assert!(fault_reached(&mut db));
    assert_eq!(ledger(&mut db), before);
    assert!(delivery(&mut db, requested.id).is_none());
    allow_audit(&mut db);
    let ready = store
        .complete(&claimed.lease, &snapshot, artifacts(&snapshot), at)
        .unwrap();
    assert_eq!(ready.ready_email, Some(CaseReportEmailState::Pending));
    let row = delivery(&mut db, requested.id).unwrap();
    assert_eq!(row["state"], "pending");
    assert!(row["message"].is_null());
    assert!(row["provider_deadline"].is_null());
    assert_eq!(
        request(&store, &actor, ready.command.clone(), at).unwrap(),
        ready
    );
    assert_eq!(delivery(&mut db, requested.id), Some(row));
}

#[test]
fn absent_or_exact_two_second_estimate_never_enqueues_ready_email() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    for estimate in [None, Some(2000)] {
        let queued = estimated_request(&mut db, &store, &actor, estimate);
        let ready = publish(&store, &queued, at);
        assert_eq!(ready.ready_email, None);
        assert!(delivery(&mut db, ready.id).is_none());
    }
    assert!(store
        .claim_delivery(claim(at), &template())
        .unwrap()
        .is_none());
}

#[test]
fn revoked_requester_is_stopped_at_claim_without_changing_ready_artifacts() {
    for revoke in ["inactive", "generation", "membership"] {
        let Some(mut db) = fixture() else { return };
        let at = db.at;
        db.case = seed_case(&mut db, "Authorized captured case", at);
        let id = db.user("litigator", true);
        let actor = principal(&mut db, id);
        let store = store(&db, clock(at));
        let ready = ready(&mut db, &store, &actor);
        let before = db.admin.query_one(
            "SELECT jsonb_agg(to_jsonb(a) ORDER BY format) FROM case_report_artifacts a WHERE report_id=$1",
            &[&ready.id.as_uuid()],
        ).unwrap().get::<_, Value>(0);
        match revoke {
            "inactive" => {
                db.admin.execute(
                "UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
                &[&id.as_uuid()],
            ).unwrap();
            }
            "generation" => {
                db.admin
                    .execute(
                        "UPDATE users SET password_hash=password_hash || '-rotated',
                 revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
                        &[&id.as_uuid()],
                    )
                    .unwrap();
            }
            _ => {
                db.admin
                    .execute(
                        "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                        &[&db.case.as_uuid(), &id.as_uuid()],
                    )
                    .unwrap();
            }
        }
        assert!(store
            .claim_delivery(claim(at), &template())
            .unwrap()
            .is_none());
        assert_eq!(
            delivery(&mut db, ready.id).unwrap()["state"],
            "access_revoked"
        );
        let row = db.admin.query_one(
            "SELECT state,(SELECT jsonb_agg(to_jsonb(a) ORDER BY format) FROM case_report_artifacts a WHERE report_id=$1)
             FROM case_report_jobs WHERE id=$1", &[&ready.id.as_uuid()],
        ).unwrap();
        assert_eq!(row.get::<_, &str>(0), "ready");
        assert_eq!(row.get::<_, Value>(1), before);
        assert!(matches!(
            store.get(&actor, ready.id, at),
            Err(ApplicationError::InvalidSession)
                | Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
        ));
    }
}

#[test]
fn permission_loss_after_claim_is_checked_again_before_provider_io() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    db.case = seed_case(&mut db, "Permission before email", at);
    let id = db.user("litigator", true);
    let actor = principal(&mut db, id);
    let store = store(&db, clock(at));
    let ready = ready(&mut db, &store, &actor);
    let lease = store
        .claim_delivery(claim(at), &template())
        .unwrap()
        .unwrap();
    assert_eq!(
        store.authorize_delivery(&lease, at).unwrap(),
        Some(lease.message.clone())
    );
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &id.as_uuid()],
        )
        .unwrap();
    assert!(store.authorize_delivery(&lease, at).unwrap().is_none());
    assert!(!store
        .finish_delivery(outcome(&lease, CaseReportEmailState::Accepted, None, at))
        .unwrap());
    assert_eq!(
        delivery(&mut db, ready.id).unwrap()["state"],
        "access_revoked"
    );
}
