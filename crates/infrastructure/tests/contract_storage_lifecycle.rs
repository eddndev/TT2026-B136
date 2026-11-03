//! The event graph of contracts in PostgreSQL: the single open-request
//! definition, the expired-closing rule, deferred pairing and the
//! notification inbox: docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use contract_storage_support::{
    flow::{insert_event, resource, Event, Request},
    in_transaction, rejection, Harness,
};
use uuid::Uuid;

fn reason(message: &str) -> (String, String) {
    ("23514".into(), message.into())
}

/// The `abandoned` event, which names no request.
fn abandon(test: &mut Harness, at: i64) -> Result<(), postgres::Error> {
    let revision = test.next_revision();
    let firm = test.firm();
    let resource = format!("contract:{}:revision:{revision}", test.contract);
    let sequence = test.audit(&firm.email, "contract.abandoned", &resource, at);
    let contract = test.contract;
    in_transaction(test, |tx| {
        insert_event(
            tx,
            &Event {
                contract,
                revision,
                kind: "abandoned",
                request: None,
                actor: Some(&firm),
                reason: Some("The parties chose another instrument"),
                at,
                operation: Uuid::new_v4(),
                sequence,
            },
        )
    })
    .map(|_| ())
}

#[test]
fn one_open_request_and_an_exhausted_one_is_closed_before_the_next() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let first = test.prepare(test.now() + 60, 1);
    test.insert_request(&first).unwrap();
    let open = test.prepare(first.created() + 3600, 7);
    assert_eq!(
        rejection(test.insert_request(&open)),
        reason("contract request is open")
    );
    let exhausted = first.expires() + 1;
    let late = test.prepare(exhausted, 7);
    assert_eq!(
        rejection(test.insert_request(&late)),
        reason("contract request must expire first")
    );
    assert_eq!(
        rejection(abandon(&mut test, exhausted)),
        reason("contract request must expire first")
    );
    let firm = test.firm();
    assert_eq!(
        rejection(test.close(
            &first,
            "cancelled",
            Some(&firm),
            Some("Late"),
            first.expires()
        )),
        ("23514".into(), "contract request has expired".into())
    );
    assert_eq!(
        rejection(test.close(&first, "expired", None, None, first.expires() - 1)),
        reason("contract request has not expired")
    );
    test.close(&first, "expired", None, None, exhausted)
        .unwrap();
    test.insert_request(&late).unwrap();
    let kinds: Vec<String> = test
        .db()
        .admin
        .query_one(
            "SELECT array_agg(kind ORDER BY revision) FROM contract_events",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(kinds, ["created", "requested", "expired", "requested"]);
}

#[test]
fn events_follow_the_request_graph_and_terminal_states_admit_nothing() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    let firm = test.firm();
    assert_eq!(
        rejection(test.close(
            &request,
            "declined",
            Some(&firm),
            None,
            request.created() + 1
        )),
        reason("contract event is out of turn")
    );
    let client = test.client.clone();
    assert_eq!(
        rejection(test.close(
            &request,
            "cancelled",
            Some(&client),
            Some("No"),
            request.created() + 1
        )),
        ("42501".into(), "contract actor is not designated".into())
    );
    test.close(
        &request,
        "rejected",
        Some(&client),
        None,
        request.created() + 2,
    )
    .unwrap();
    abandon(&mut test, request.created() + 3).unwrap();
    let again = test.prepare(request.created() + 4, 7);
    assert_eq!(
        rejection(test.insert_request(&again)),
        reason("contract is closed")
    );
}

#[test]
fn a_completed_contract_admits_no_new_request() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    test.sign_client(&request, request.created() + 60).unwrap();
    test.sign_firm(&request, request.created() + 120).unwrap();
    let again = test.prepare(request.created() + 180, 7);
    assert_eq!(
        rejection(test.insert_request(&again)),
        reason("contract is closed")
    );
}

/// Commits one event with no paired row; the deferred trigger refuses it.
fn unpaired(test: &mut Harness, request: &Request, kind: &str, actor_is_client: bool) {
    let revision = test.next_revision();
    let actor = if actor_is_client {
        test.client.clone()
    } else {
        test.firm()
    };
    let at = request.created() + 60 * revision;
    let sequence = test.audit(
        &actor.email,
        &format!("contract.{kind}"),
        &resource(request.id, revision),
        at,
    );
    let contract = test.contract;
    let result = in_transaction(test, |tx| {
        insert_event(
            tx,
            &Event {
                contract,
                revision,
                kind,
                request: Some(request.id),
                actor: Some(&actor),
                reason: None,
                at,
                operation: Uuid::new_v4(),
                sequence,
            },
        )
    });
    assert_eq!(
        rejection(result),
        reason("contract record lacks its paired rows")
    );
}

#[test]
fn signed_events_commit_only_with_their_signature_and_completion() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    unpaired(&mut test, &request, "client_signed", true);
    test.sign_client(&request, request.created() + 60).unwrap();
    unpaired(&mut test, &request, "completed", false);
    let counts: (i64, i64) = {
        let row = test
            .db()
            .admin
            .query_one(
                "SELECT (SELECT count(*) FROM contract_events),
                (SELECT count(*) FROM contract_party_signatures)",
                &[],
            )
            .unwrap();
        (row.get(0), row.get(1))
    };
    assert_eq!(counts, (3, 1));
}

#[test]
fn a_contract_root_or_request_without_its_event_does_not_commit() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let other = test.upload("otro.pdf");
    let firm = test.firm();
    let at = test.now();
    let case = test.individual.db.case.as_uuid();
    let result = in_transaction(&test, |tx| {
        tx.execute(
            "INSERT INTO contracts(id,case_id,document_id,title,created_by,created_by_email,
            created_at_seconds,created_at_nanoseconds) VALUES($1,$2,$3,'Sin evento',$4,$5,$6,0)",
            &[
                &Uuid::new_v4(),
                &case,
                &other.id.as_uuid(),
                &firm.id.as_uuid(),
                &firm.email,
                &at,
            ],
        )
    });
    assert_eq!(
        rejection(result),
        reason("contract record lacks its paired rows")
    );
    let request = test.prepare(test.now() + 60, 7);
    let result = in_transaction(&test, |tx| test.insert_request_row(tx, &request));
    assert_eq!(
        rejection(result),
        reason("contract record lacks its paired rows")
    );
    test.insert_request(&request).unwrap();
}

fn notify(test: &Harness, request: &Request, recipient: Uuid) -> Result<u64, postgres::Error> {
    let contract = test.contract;
    in_transaction(test, |tx| {
        tx.execute(
            "INSERT INTO contract_notifications(id,request_id,contract_id,event_revision,
            recipient_id,kind,message_from,message_to,subject,message_text,created_at_seconds,
            delivery_state,delivery_generation)
            SELECT $1,$2,$3,e.revision,u.id,'requested','qadra@example.test',u.email,
                'Solicitud de firma','Tiene un contrato pendiente de firma.',e.at_seconds,'pending',0
            FROM contract_events e,users u WHERE e.request_id=$2 AND e.kind='requested' AND u.id=$4",
            &[&Uuid::new_v4(), &request.id, &contract, &recipient],
        )
    })
}

#[test]
fn notifications_reach_only_the_recipients_of_their_event_and_change_only_delivery() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    assert_eq!(
        rejection(notify(&test, &request, test.firm().id.as_uuid())),
        reason("contract notification differs from its event")
    );
    assert_eq!(
        notify(&test, &request, test.client.id.as_uuid()).unwrap(),
        1
    );
    let mut runtime = test.runtime();
    runtime
        .execute(
            "UPDATE contract_notifications SET delivery_state='sent',delivery_generation=1",
            &[],
        )
        .unwrap();
    assert_eq!(
        rejection(runtime.execute("UPDATE contract_notifications SET subject='Otro'", &[])).0,
        "42501"
    );
    assert_eq!(
        rejection(
            test.db()
                .admin
                .execute("UPDATE contract_notifications SET subject='Otro'", &[])
        ),
        reason("contract notification content is immutable")
    );
    assert_eq!(
        rejection(
            test.db()
                .admin
                .execute("DELETE FROM contract_notifications", &[])
        ),
        reason("contract history is immutable")
    );
}
