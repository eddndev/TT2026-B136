use super::{case_administration_support::Fixture, procedural_fact_sql_support::*};
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn empty() -> Value {
    json!({"resolution":null,"participants":[],"hearing_results":[],"direct_supports":[]})
}
pub fn resolution() -> Value {
    fixture("resolution_minimum")["normalized"].clone()
}
pub fn notification(parent: Uuid) -> Value {
    let mut value = fixture("notification_minimum")["normalized"].clone();
    value["resolution"]["id"] = json!(parent);
    value
}
pub fn check(db: &mut Fixture, family: &str, values: &Value, sources: &Value, accepted: bool) {
    let result = db.admin.query_one(
        "SELECT validate_procedural_fact_sources($1,$2,$3,$4)",
        &[&db.case.as_uuid(), &family, values, sources],
    );
    if accepted {
        result.unwrap();
    } else {
        let error = result.unwrap_err();
        assert_eq!(
            error.code().map(|value| value.code()),
            Some("23514"),
            "{error}"
        );
    }
}
pub fn parent(db: &mut Fixture) -> (Uuid, Value) {
    let id = Uuid::new_v4();
    let operation = Uuid::new_v4();
    let values = bytes(fixture("resolution_minimum")["hex"].as_str().unwrap());
    let sources = bytes(receipts()["sources"][0]["hex"].as_str().unwrap());
    let mut receipt = b"PFTXN1".to_vec();
    receipt.extend(operation.as_bytes());
    receipt.extend(db.owner.as_uuid().as_bytes());
    receipt.extend(db.case.as_uuid().as_bytes());
    receipt.push(0);
    receipt.extend(id.as_bytes());
    receipt.push(0);
    receipt.extend(0u32.to_be_bytes());
    receipt.extend(RingSha256Hasher.hash_bytes(&values).as_bytes());
    receipt.extend(RingSha256Hasher.hash_bytes(&sources).as_bytes());
    receipt.push(0);
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO case_procedural_facts(family,id,case_id) VALUES('resolution',$1,$2)",
        &[&id, &db.case.as_uuid()],
    )
    .unwrap();
    tx.execute("INSERT INTO case_procedural_fact_revisions(family,id,case_id,revision,values_canonical,values_digest,sources_canonical,sources_digest,operation_id,action,submission_canonical,submission_digest,recorded_administration_title,recorded_administration_reference,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email) VALUES('resolution',$1,$2,1,$3,sha256($3),$4,sha256($4),$5,'record',$6,sha256($6),'Baseline','REF-OLD',1735689600,0,$7,'owner@example.test')", &[&id,&db.case.as_uuid(),&values,&sources,&operation,&receipt,&db.owner.as_uuid()]).unwrap();
    tx.commit().unwrap();
    let projection = json!({"case":db.case.to_string(),"id":id,"revision":1,
        "values_digest":RingSha256Hasher.hash_bytes(&values).to_hex(),
        "submission_digest":RingSha256Hasher.hash_bytes(&receipt).to_hex(),"status":"recorded",
        "class":{"known":{"kind":"order"}},"issuer":{"known":"x"},
        "issued_at":{"precision":"unknown"},"summary":"x"});
    (id, projection)
}
pub fn manual(db: &mut Fixture) -> (Uuid, Value) {
    let id = Uuid::new_v4();
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
        &[&id, &db.case.as_uuid()],
    )
    .unwrap();
    tx.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,directory_status,values_digest,changed_at,changed_by,changed_by_email) VALUES($1,1,'Person','Declared role','active',sha256(participant_values_bytes('Person','Declared role',NULL,NULL,'active')),'2025-01-01T00:00:00Z',$2,'owner@example.test')", &[&id,&db.owner.as_uuid()]).unwrap();
    let digest: String = tx.query_one("SELECT encode(values_digest,'hex') FROM case_participant_revisions WHERE participant_id=$1 AND revision=1", &[&id]).unwrap().get(0);
    tx.commit().unwrap();
    (
        id,
        json!({"case":db.case.to_string(),"id":id,"revision":1,"values_digest":digest,
        "status":"active","subject":null,"kind":null,"display_name":"Person",
        "procedural_role":"Declared role","organization":null}),
    )
}
pub fn document(db: &mut Fixture) -> (Value, Value) {
    let id = Uuid::new_v4();
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO document_series(id,case_id,first_available_version) VALUES($1,$2,1)",
        &[&id, &db.case.as_uuid()],
    )
    .unwrap();
    tx.execute("INSERT INTO documents(id,version,case_id,name,digest,vault) VALUES($1,1,$2,'source.pdf',$3,$4)", &[&id,&db.case.as_uuid(),&&[17u8;32][..],&vec![0u8]]).unwrap();
    tx.commit().unwrap();
    (
        json!({"document_id":id,"version":1,"digest":"11".repeat(32),"locator":"page 1"}),
        json!({"id":id,"version":1,"digest":"11".repeat(32),"name":"source.pdf","format":"pdf","policy":"pdf_docx_v1"}),
    )
}
