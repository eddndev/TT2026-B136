#![allow(dead_code)]
use crate::alert_backend_support::{self as alerts, *};
use application::alerts::*;
use infrastructure::{PostgresAlertStore, RingSha256Hasher};
use std::sync::Arc;
use time::Duration;

pub struct DeliveryFixture {
    pub db: Fixture,
    pub clock: Arc<MutableClock>,
    pub store: PostgresAlertStore,
    pub alert: AlertId,
}
pub fn prepared() -> Option<DeliveryFixture> {
    let mut db = fixture()?;
    db.admin
        .execute(
            "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2) ON CONFLICT DO NOTHING",
            &[&db.case.as_uuid(), &db.owner.as_uuid()],
        )
        .unwrap();
    hearing(
        &db,
        db.at.replace_nanosecond(0).unwrap() + Duration::hours(47),
    );
    let clock = Arc::new(MutableClock::new(db.at));
    let store = alerts::store(&db, clock.clone());
    drive(&store);
    let page = store.list(db.owner, query(20)).unwrap();
    assert_eq!(page.alerts.len(), 1);
    let alert = page.alerts[0].id;
    Some(DeliveryFixture {
        db,
        clock,
        store,
        alert,
    })
}
pub fn completion(
    claim: &AlertDeliveryClaim,
    outcome: AlertEmailOutcome,
) -> AlertDeliveryCompletion {
    AlertDeliveryCompletion {
        delivery_id: claim.delivery_id,
        claim_id: claim.claim_id,
        attempt: claim.attempt,
        outcome,
    }
}
pub fn unknown() -> AlertEmailOutcome {
    AlertEmailOutcome::Unknown {
        code: "transport_uncertain".into(),
    }
}
pub fn accepted() -> AlertEmailOutcome {
    AlertEmailOutcome::Accepted {
        provider_id: "c27a8c17-45ba-4d73-bc96-3c226742a19e".into(),
    }
}
pub fn reopened(fixture: &DeliveryFixture) -> PostgresAlertStore {
    PostgresAlertStore::open(
        &fixture.db.runtime_url,
        Arc::new(RingSha256Hasher),
        fixture.clock.clone(),
        Some(AlertEmailConfiguration {
            from_email: "new-sender@example.test".into(),
            login_url: "https://new.example.test/login".into(),
        }),
    )
    .unwrap()
}
pub fn ledger(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'outbox',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM alert_email_outbox r),
        'attempts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY delivery_id,sequence) FROM alert_email_attempts r),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0)
}
