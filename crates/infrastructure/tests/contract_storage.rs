//! Bilateral contract storage against PostgreSQL: the full path through the
//! runtime role, the SQL statement builders against the shared vectors,
//! append-only history, expected revisions, audit references and the
//! startup recheck: docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use contract_storage_support::{
    flow::{insert_event, resource, Event},
    in_transaction, opens, rejection, tamper, Harness,
};
use serde_json::Value;
use uuid::Uuid;

const VECTORS: &str = include_str!("../../domain/tests/fixtures/contract_statement_vectors.json");

#[test]
fn a_request_is_signed_by_both_parties_and_completed_through_the_runtime_role() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    test.sign_client(&request, request.created() + 60).unwrap();
    test.sign_firm(&request, request.created() + 120).unwrap();
    let row = test
        .db()
        .admin
        .query_one(
            "SELECT array_agg(kind ORDER BY revision),
            (SELECT count(*) FROM contract_party_signatures),
            (SELECT count(*) FROM contract_completions),
            (SELECT count(*) FROM audit_events WHERE action LIKE 'contract.%')
            FROM contract_events",
            &[],
        )
        .unwrap();
    assert_eq!(
        row.get::<_, Vec<String>>(0),
        ["created", "requested", "client_signed", "completed"]
    );
    assert_eq!(
        (
            row.get::<_, i64>(1),
            row.get::<_, i64>(2),
            row.get::<_, i64>(3)
        ),
        (2, 1, 4)
    );
    assert!(opens(&test));
    test.db().migrate();
    assert!(opens(&test));
}

fn text(value: &Value) -> String {
    value.as_str().unwrap().to_owned()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn the_sql_builders_reproduce_the_shared_statement_vectors_for_the_runtime_role() {
    let Some(test) = Harness::new() else {
        return;
    };
    let vectors: Value = serde_json::from_str(VECTORS).unwrap();
    let inputs = &vectors["request_inputs"];
    let mut runtime = test.runtime();
    let request: Vec<u8> = runtime
        .query_one(
            "SELECT contract_request_bytes($1::text::uuid,$2::text::uuid,$3::text::uuid,
            $4::text::uuid,$5,decode($6,'hex'),$7::text::uuid,$8::text::uuid,$9::text::uuid,
            decode($10,'hex'),decode($11,'hex'),$12,$13)",
            &[
                &text(&inputs["deployment_id"]),
                &text(&inputs["request_id"]),
                &text(&inputs["case_id"]),
                &text(&inputs["document_id"]),
                &inputs["document_version"].as_i64().unwrap(),
                &text(&inputs["document_sha256"]),
                &text(&inputs["client_account_id"]),
                &text(&inputs["firm_account_id"]),
                &text(&inputs["requester_account_id"]),
                &text(&inputs["client_certificate_sha256"]),
                &text(&inputs["firm_certificate_sha256"]),
                &inputs["created_at_seconds"].as_i64().unwrap(),
                &inputs["expires_at_seconds"].as_i64().unwrap(),
            ],
        )
        .unwrap()
        .get(0);
    assert_eq!(request.len(), 240);
    assert_eq!(to_hex(&request), text(&vectors["request"]["hex"]));
    for (party, statement) in [
        ("client_party", "client_statement"),
        ("firm_party", "firm_statement"),
    ] {
        let p = &vectors[party];
        let bytes: Vec<u8> = runtime
            .query_one(
                "SELECT contract_party_bytes($1::text::uuid,decode($2,'hex'),$3,$4::text::uuid,
                $5,$6,$7::text::uuid,decode($8,'hex'),$9::text::uuid,$10::text::uuid,
                decode($11,'hex'),decode($12,'hex'),$13,decode($14,'hex'),decode($15,'hex'))",
                &[
                    &text(&inputs["deployment_id"]),
                    &text(&p["root_sha256"]),
                    &p["trust_revision"].as_i64().unwrap(),
                    &text(&p["account_id"]),
                    &p["account_revision"].as_i64().unwrap(),
                    &p["authentication_generation"].as_i64().unwrap(),
                    &text(&p["binding_id"]),
                    &text(&p["certificate_sha256"]),
                    &text(&p["operation_id"]),
                    &text(&inputs["request_id"]),
                    &text(&vectors["request"]["sha256"]),
                    &text(&inputs["document_sha256"]),
                    &i16::try_from(p["position"].as_i64().unwrap()).unwrap(),
                    &text(&p["prior_signature_sha256"]),
                    &text(&p["prior_token_sha256"]),
                ],
            )
            .unwrap()
            .get(0);
        assert_eq!(bytes.len(), 303);
        assert_eq!(to_hex(&bytes), text(&vectors[statement]["hex"]), "{party}");
    }
}

#[test]
fn stored_statements_must_be_the_rebuilt_bytes() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let mut request = test.prepare(test.now() + 60, 7);
    request.request = test.prepare(test.now() + 61, 7).request;
    assert_eq!(
        rejection(test.insert_request(&request)),
        ("23514".into(), "contract request bytes differ".into())
    );
}

#[test]
fn contract_history_rejects_every_update_delete_and_truncate() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    test.sign_client(&request, request.created() + 60).unwrap();
    test.sign_firm(&request, request.created() + 120).unwrap();
    for (table, column) in [
        ("contracts", "title"),
        ("contract_signature_requests", "ceremony_reference"),
        ("contract_events", "actor_label"),
        ("contract_party_signatures", "actor_email"),
        ("contract_completions", "tsa_chain_pem"),
    ] {
        let mut runtime = test.runtime();
        let update = format!("UPDATE {table} SET {column}={column}");
        assert_eq!(rejection(runtime.execute(update.as_str(), &[])).0, "42501");
        for sql in [
            update.clone(),
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table} CASCADE"),
        ] {
            assert_eq!(
                rejection(test.db().admin.execute(sql.as_str(), &[])),
                ("23514".into(), "contract history is immutable".into()),
                "{sql}"
            );
        }
    }
    let count: i64 = test
        .db()
        .admin
        .query_one("SELECT count(*) FROM contract_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 4);
}

#[test]
fn an_event_must_carry_the_next_revision_of_its_contract() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let request = test.prepare(test.now() + 60, 7);
    test.insert_request(&request).unwrap();
    let firm = test.firm();
    let at = request.created() + 60;
    for revision in [2, 4] {
        let sequence = test.audit(
            &firm.email,
            "contract.cancelled",
            &resource(request.id, revision),
            at,
        );
        let contract = test.contract;
        let result = in_transaction(&test, |tx| {
            insert_event(
                tx,
                &Event {
                    contract,
                    revision,
                    kind: "cancelled",
                    request: Some(request.id),
                    actor: Some(&firm),
                    reason: Some("Duplicated"),
                    at,
                    operation: Uuid::new_v4(),
                    sequence,
                },
            )
        });
        assert_eq!(
            rejection(result),
            ("40001".into(), "contract revision conflict".into())
        );
    }
    test.close(&request, "cancelled", Some(&firm), Some("Duplicated"), at)
        .unwrap();
}

#[test]
fn audit_rows_accept_exactly_the_contract_event_reference_among_twenty_eight_triggers() {
    let Some(mut test) = Harness::new() else {
        return;
    };
    let count: i64 = test
        .db()
        .admin
        .query_one(
            "SELECT count(*) FROM pg_trigger WHERE tgrelid='audit_events'::regclass",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 28);
    assert!(opens(&test));
    test.db()
        .admin
        .batch_execute(
            "ALTER TABLE contract_events DROP CONSTRAINT contract_events_audit_sequence_fkey",
        )
        .unwrap();
    assert!(!opens(&test));
}

#[test]
fn the_startup_recheck_rejects_a_changed_signature_or_manifest() {
    for target in ["signature", "manifest"] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        let request = test.prepare(test.now() + 60, 7);
        test.insert_request(&request).unwrap();
        test.sign_client(&request, request.created() + 60).unwrap();
        test.sign_firm(&request, request.created() + 120).unwrap();
        assert!(opens(&test));
        tamper(
            &mut test,
            if target == "signature" {
                "UPDATE contract_party_signatures
                SET signature=set_byte(signature,8,get_byte(signature,8) # 1) WHERE \"position\"=2"
            } else {
                "UPDATE contract_completions SET manifest=manifest || ' '::bytea,
                manifest_sha256=sha256(manifest || ' '::bytea)"
            },
        );
        assert!(!opens(&test), "{target}");
    }
}

#[test]
fn startup_rejects_an_altered_contract_catalog_or_a_widened_runtime_role() {
    for change in [
        "ALTER TABLE contract_events DISABLE TRIGGER contract_event_register",
        "GRANT UPDATE(kind) ON contract_events TO {role}",
        "REVOKE EXECUTE ON FUNCTION contract_party_bytes(uuid,bytea,bigint,uuid,bigint,bigint,
            uuid,bytea,uuid,uuid,bytea,bytea,smallint,bytea,bytea) FROM {role}",
    ] {
        let Some(mut test) = Harness::new() else {
            return;
        };
        assert!(opens(&test));
        let role = test.db().role.clone();
        test.db()
            .admin
            .batch_execute(&change.replace("{role}", &role))
            .unwrap();
        assert!(!opens(&test), "{change}");
    }
}
