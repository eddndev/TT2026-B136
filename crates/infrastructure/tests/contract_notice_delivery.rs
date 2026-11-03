//! Leased e-mail delivery of contract notices over PostgreSQL: only the
//! delivery columns change, a live lease fences every outcome and an
//! inactive recipient is never claimed:
//! docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_service_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::contracts::*;
use contract_service_support::Live;
use uuid::Uuid;

fn claim(live: &Live) -> Option<ContractNoticeLease> {
    live.store()
        .claim_notice(ContractNoticeClaim {
            lease_id: Uuid::new_v4(),
            lease_seconds: 30,
            provider_window_seconds: 86_400,
        })
        .unwrap()
}

fn finish(live: &Live, lease: &ContractNoticeLease, state: ContractNoticeState) -> bool {
    live.store()
        .finish_notice(ContractNoticeFinish {
            id: lease.id,
            generation: lease.generation,
            lease_id: lease.lease_id,
            state,
            retry_at: None,
        })
        .unwrap()
}

fn delivery(live: &mut Live, id: Uuid) -> (String, Option<i64>, Option<i64>, i64) {
    let row = live
        .test
        .db()
        .admin
        .query_one(
            "SELECT delivery_state,retry_at,provider_deadline,delivery_generation
            FROM contract_notifications WHERE id=$1",
            &[&id],
        )
        .unwrap();
    (row.get(0), row.get(1), row.get(2), row.get(3))
}

fn admin(live: &mut Live, sql: &str, id: Uuid) -> Result<u64, postgres::Error> {
    live.test.db().admin.execute(sql, &[&id])
}

#[test]
fn a_notice_is_leased_once_and_its_content_never_changes() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let lease = claim(&live).expect("the request notice of the Client");
    assert_eq!(lease.message.to, live.client().email);
    assert_eq!(lease.message.from, "qadra@example.test");
    assert!(lease
        .message
        .text
        .contains("https://qadra.example.test#contracts"));
    assert_eq!(lease.generation, 1);
    assert!(claim(&live).is_none(), "a live lease is claimed twice");
    let refused = admin(
        &mut live,
        "UPDATE contract_notifications SET message_text='changed' WHERE id=$1",
        lease.id,
    )
    .unwrap_err();
    assert_eq!(
        refused.as_db_error().unwrap().message(),
        "contract notification content is immutable"
    );
    let mut runtime =
        postgres::Client::connect(&live.test.individual.db.runtime_url, postgres::NoTls).unwrap();
    assert!(runtime
        .execute(
            "UPDATE contract_notifications SET subject='changed' WHERE id=$1",
            &[&lease.id],
        )
        .is_err());
    assert!(finish(&live, &lease, ContractNoticeState::Sent));
    assert!(
        !finish(&live, &lease, ContractNoticeState::Sent),
        "replayed lease"
    );
    let (state, retry, deadline, generation) = delivery(&mut live, lease.id);
    assert_eq!((state.as_str(), retry, generation), ("sent", None, 1));
    assert_eq!(deadline, Some(lease.provider_deadline.unix_timestamp()));
    assert!(claim(&live).is_none(), "a sent notice is claimed again");
}

#[test]
fn an_uncertain_notice_keeps_its_deadline_and_an_inactive_recipient_waits() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let first = claim(&live).unwrap();
    assert!(finish(&live, &first, ContractNoticeState::Uncertain));
    let (state, retry, deadline, _) = delivery(&mut live, first.id);
    assert_eq!(state, "uncertain");
    assert_eq!(
        retry, deadline,
        "an uncertain attempt without retry is final"
    );
    assert!(claim(&live).is_none());
    admin(
        &mut live,
        "UPDATE contract_notifications SET retry_at=0 WHERE id=$1",
        first.id,
    )
    .unwrap();
    let second = claim(&live).unwrap();
    assert_eq!((second.id, second.generation), (first.id, 2));
    assert_eq!(second.provider_deadline, first.provider_deadline);
    assert!(finish(&live, &second, ContractNoticeState::Rejected));
    admin(
        &mut live,
        "UPDATE contract_notifications SET delivery_state='pending',retry_at=NULL WHERE id=$1",
        first.id,
    )
    .unwrap();
    let client = live.client().id.as_uuid();
    let access = "UPDATE users SET active=NOT active,revision=revision+1,
        auth_generation=auth_generation+1 WHERE id=$1";
    admin(&mut live, access, client).unwrap();
    assert!(claim(&live).is_none(), "an inactive recipient was claimed");
    admin(&mut live, access, client).unwrap();
    assert_eq!(claim(&live).unwrap().generation, 3);
}
