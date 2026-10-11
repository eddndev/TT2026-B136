use super::*;
use std::sync::Barrier;

#[test]
fn concurrent_claims_share_one_delivery_and_fence_the_expired_worker() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = store(&db, timer.clone());
    let ready = ready(&mut db, &initial, &actor);
    let stores = [store(&db, timer.clone()), store(&db, timer.clone())];
    let barrier = Arc::new(Barrier::new(2));
    let leases = std::thread::scope(|scope| {
        let tasks: Vec<_> = stores
            .into_iter()
            .map(|store| {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    store.claim_delivery(claim(at), &template()).unwrap()
                })
            })
            .collect();
        tasks
            .into_iter()
            .filter_map(|task| task.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(leases.len(), 1);
    let first = &leases[0];
    assert_eq!(first.report_id, ready.id);
    let later = first.lease_until + Duration::seconds(1);
    timer.set(later);
    let second = initial
        .claim_delivery(claim(later), &template())
        .unwrap()
        .unwrap();
    assert_eq!(second.delivery_id, first.delivery_id);
    assert_eq!(second.message, first.message);
    assert_eq!(second.provider_deadline, first.provider_deadline);
    assert_eq!(second.generation, first.generation + 1);
    assert_ne!(second.lease_id, first.lease_id);
    assert!(initial.authorize_delivery(first, later).unwrap().is_none());
    assert!(!initial
        .finish_delivery(outcome(first, CaseReportEmailState::Accepted, None, later))
        .unwrap());
    let accepted = outcome(&second, CaseReportEmailState::Accepted, None, later);
    for field in ["report", "delivery", "generation", "lease"] {
        let mut stale = accepted.clone();
        match field {
            "report" => stale.report_id = CaseReportId::new(),
            "delivery" => stale.delivery_id = Uuid::new_v4(),
            "generation" => stale.generation += 1,
            _ => stale.lease_id = Uuid::new_v4(),
        }
        assert!(!initial.finish_delivery(stale).unwrap());
    }
    assert!(initial.finish_delivery(accepted.clone()).unwrap());
    assert!(!initial.finish_delivery(accepted).unwrap());
    assert!(initial
        .claim_delivery(claim(later), &template())
        .unwrap()
        .is_none());
    assert_eq!(
        initial.get(&actor, ready.id, later).unwrap().ready_email,
        Some(CaseReportEmailState::Accepted)
    );
}

#[test]
fn retry_preserves_first_envelope_and_deadline_across_reopen_and_template_change() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = store(&db, timer.clone());
    ready(&mut db, &initial, &actor);
    let first = initial
        .claim_delivery(claim(at), &template())
        .unwrap()
        .unwrap();
    assert_eq!(first.message.to, actor.email);
    assert_eq!(first.message.from, template().from);
    assert!(!first.message.text.contains(&first.report_id.to_string()));
    let retry_at = at + Duration::seconds(60);
    assert!(initial
        .finish_delivery(outcome(
            &first,
            CaseReportEmailState::Uncertain,
            Some(retry_at),
            at
        ))
        .unwrap());
    assert!(initial
        .claim_delivery(claim(at), &template())
        .unwrap()
        .is_none());
    drop(initial);
    timer.set(retry_at);
    let reopened = store(&db, timer);
    let changed = CaseReportEmailTemplate {
        from: "changed@example.test".into(),
        subject: "Changed settings".into(),
        text: "https://changed.example.test/#reports".into(),
    };
    let second = reopened
        .claim_delivery(claim(retry_at), &changed)
        .unwrap()
        .unwrap();
    assert_eq!(second.delivery_id, first.delivery_id);
    assert_eq!(second.provider_deadline, first.provider_deadline);
    assert_eq!(second.message, first.message);
    let mut altered = second.clone();
    altered.message.to = "foreign@example.test".into();
    altered.message.text = "Caller-supplied replacement".into();
    assert_eq!(
        reopened.authorize_delivery(&altered, retry_at).unwrap(),
        Some(first.message)
    );
}

#[test]
fn retry_and_lease_cannot_extend_the_first_provider_window() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let timer = clock(at);
    let store = store(&db, timer.clone());
    ready(&mut db, &store, &actor);
    let mut first_claim = claim(at);
    first_claim.provider_window_seconds = 90;
    let first = store
        .claim_delivery(first_claim, &template())
        .unwrap()
        .unwrap();
    assert!(store
        .finish_delivery(outcome(
            &first,
            CaseReportEmailState::Uncertain,
            Some(first.provider_deadline),
            at
        ))
        .is_err());
    let retry_at = at + Duration::seconds(60);
    assert!(store
        .finish_delivery(outcome(
            &first,
            CaseReportEmailState::Uncertain,
            Some(retry_at),
            at
        ))
        .unwrap());
    let near_deadline = first.provider_deadline - Duration::seconds(1);
    timer.set(near_deadline);
    let second = store
        .claim_delivery(claim(near_deadline), &template())
        .unwrap()
        .unwrap();
    assert_eq!(second.provider_deadline, first.provider_deadline);
    assert_eq!(second.lease_until, first.provider_deadline);
    timer.set(first.provider_deadline);
    assert!(store
        .authorize_delivery(&second, first.provider_deadline)
        .unwrap()
        .is_none());
    assert!(!store
        .finish_delivery(outcome(
            &second,
            CaseReportEmailState::Accepted,
            None,
            first.provider_deadline
        ))
        .unwrap());
    assert!(store
        .claim_delivery(claim(first.provider_deadline), &template())
        .unwrap()
        .is_none());
}

#[test]
fn rejection_and_uncertainty_without_retry_are_terminal() {
    for terminal in [
        CaseReportEmailState::Rejected,
        CaseReportEmailState::Uncertain,
    ] {
        let Some(mut db) = fixture() else { return };
        let at = db.at;
        let actor = owner(&mut db);
        let timer = clock(at);
        let store = store(&db, timer.clone());
        let ready = ready(&mut db, &store, &actor);
        let lease = store
            .claim_delivery(claim(at), &template())
            .unwrap()
            .unwrap();
        assert!(store
            .finish_delivery(outcome(&lease, terminal, None, at))
            .unwrap());
        let later = at + Duration::seconds(60);
        timer.set(later);
        assert!(store
            .claim_delivery(claim(later), &template())
            .unwrap()
            .is_none());
        let detail = store.get(&actor, ready.id, later).unwrap();
        assert!(matches!(detail.state, CaseReportState::Ready { .. }));
        assert_eq!(detail.ready_email, Some(terminal));
    }
}

#[test]
fn exhausted_provider_window_closes_an_abandoned_pending_lease() {
    exhausted_window(false);
}

#[test]
fn exhausted_provider_window_clears_an_uncertain_retry() {
    exhausted_window(true);
}

fn exhausted_window(retry: bool) {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let timer = clock(at);
    let store = store(&db, timer.clone());
    let ready = ready(&mut db, &store, &actor);
    let mut first_claim = claim(at);
    first_claim.provider_window_seconds = 90;
    let lease = store
        .claim_delivery(first_claim, &template())
        .unwrap()
        .unwrap();
    if retry {
        assert!(store
            .finish_delivery(outcome(
                &lease,
                CaseReportEmailState::Uncertain,
                Some(at + Duration::seconds(60)),
                at
            ))
            .unwrap());
    }
    let original = delivery(&mut db, ready.id).unwrap();
    let before = audits(&mut db, "case_report.email_finish");
    timer.set(lease.provider_deadline);
    assert!(store
        .claim_delivery(claim(lease.provider_deadline), &template())
        .unwrap()
        .is_none());
    let closed = delivery(&mut db, ready.id).unwrap();
    assert_eq!(closed["state"], "uncertain");
    assert!(closed["lease_id"].is_null());
    assert!(closed["lease_until"].is_null());
    assert!(closed["retry_at"].is_null());
    for field in ["delivery_id", "generation", "message", "provider_deadline"] {
        assert_eq!(closed[field], original[field]);
    }
    assert_eq!(audits(&mut db, "case_report.email_finish"), before + 1);
    let detail = store
        .get(&actor, ready.id, lease.provider_deadline)
        .unwrap();
    assert_eq!(detail.state, ready.state);
    assert_eq!(detail.ready_email, Some(CaseReportEmailState::Uncertain));
    assert!(!store
        .finish_delivery(outcome(
            &lease,
            CaseReportEmailState::Accepted,
            None,
            lease.provider_deadline
        ))
        .unwrap());
    assert!(store
        .claim_delivery(claim(lease.provider_deadline), &template())
        .unwrap()
        .is_none());
    assert_eq!(delivery(&mut db, ready.id), Some(closed));
    assert_eq!(audits(&mut db, "case_report.email_finish"), before + 1);
}
