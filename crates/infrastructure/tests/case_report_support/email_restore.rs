use super::*;
use postgres::{Client, NoTls};

#[test]
fn restored_delivery_keeps_message_identity_deadline_and_ready_evidence() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = store(&db, timer.clone());
    let ready = ready(&mut db, &initial, &actor);
    let pdf = initial
        .download(&actor, ready.id, CaseReportFormat::Pdf, at)
        .unwrap()
        .artifact;
    let first = initial
        .claim_delivery(claim(at), &template())
        .unwrap()
        .unwrap();
    let retry_at = at + Duration::seconds(60);
    assert!(initial
        .finish_delivery(outcome(
            &first,
            CaseReportEmailState::Uncertain,
            Some(retry_at),
            at
        ))
        .unwrap());
    let original_delivery = delivery(&mut db, ready.id).unwrap();
    let original_ledger = ledger(&mut db);
    drop(initial);
    let restored_db = backup::RestoredDatabase::copy(&db);
    let mut admin = Client::connect(&restored_db.admin_url, NoTls).unwrap();
    assert_eq!(
        admin
            .query_one(
                "SELECT to_jsonb(d) FROM case_report_email_deliveries d WHERE report_id=$1",
                &[&ready.id.as_uuid()],
            )
            .unwrap()
            .get::<_, Value>(0),
        original_delivery
    );
    timer.set(retry_at);
    let restored = open_url(&restored_db.runtime_url, timer);
    let changed = CaseReportEmailTemplate {
        from: "restored@example.test".into(),
        subject: "Different template after restore".into(),
        text: "https://restored.example.test/#reports".into(),
    };
    let resumed = restored
        .claim_delivery(claim(retry_at), &changed)
        .unwrap()
        .unwrap();
    assert_eq!(resumed.delivery_id, first.delivery_id);
    assert_eq!(resumed.message, first.message);
    assert_eq!(resumed.provider_deadline, first.provider_deadline);
    assert!(restored
        .finish_delivery(outcome(
            &resumed,
            CaseReportEmailState::Accepted,
            None,
            retry_at
        ))
        .unwrap());
    assert!(restored
        .claim_delivery(claim(retry_at), &changed)
        .unwrap()
        .is_none());
    assert_eq!(
        restored
            .download(&actor, ready.id, CaseReportFormat::Pdf, retry_at)
            .unwrap()
            .artifact,
        pdf
    );
    assert_eq!(delivery(&mut db, ready.id), Some(original_delivery));
    assert_eq!(ledger(&mut db), original_ledger);
    drop(restored);
    drop(admin);
    drop(restored_db);
}
