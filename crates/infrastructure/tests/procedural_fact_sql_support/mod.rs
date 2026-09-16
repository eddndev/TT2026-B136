use postgres::Client;
use serde_json::Value;

pub fn bytes(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
pub fn values() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../domain/tests/fixtures/procedural_fact_vectors.json"
    ))
    .unwrap()
}
pub fn receipts() -> Value {
    serde_json::from_str(include_str!(
        "../../../application/tests/fixtures/procedural-fact-receipts.json"
    ))
    .unwrap()
}
pub fn fixture(name: &str) -> Value {
    values().into_iter().find(|v| v["name"] == name).unwrap()
}
pub fn value(client: &mut Client, family: &str, bytes: &[u8]) -> Value {
    client
        .query_one("SELECT procedural_fact_values($1,$2)", &[&family, &bytes])
        .unwrap()
        .get(0)
}
pub fn rejected(client: &mut Client, function: &str, family: Option<&str>, bytes: &[u8]) {
    let result = if let Some(family) = family {
        client.query_one("SELECT procedural_fact_values($1,$2)", &[&family, &bytes])
    } else {
        assert!(matches!(
            function,
            "procedural_fact_sources" | "procedural_fact_submission"
        ));
        client.query_one(&format!("SELECT {function}($1)"), &[&bytes])
    };
    let error = result.unwrap_err();
    assert_eq!(
        error.code().map(|code| code.code()),
        Some("23514"),
        "{error}"
    );
}
