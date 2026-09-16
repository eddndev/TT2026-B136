use application::{procedural_facts::ProceduralFactError, ApplicationError};
use infrastructure::procedural_fact_codec;
use serde_json::Value;

pub fn value_vectors() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../domain/tests/fixtures/procedural_fact_vectors.json"
    ))
    .unwrap()
}
pub fn source_vectors() -> Vec<Value> {
    let value: Value = serde_json::from_str(include_str!(
        "../../../application/tests/fixtures/procedural-fact-receipts.json"
    ))
    .unwrap();
    value["sources"].as_array().unwrap().clone()
}
pub fn value_vector(name: &str) -> Value {
    value_vectors()
        .into_iter()
        .find(|v| v["name"] == name)
        .unwrap()
}
pub fn source_vector(name: &str) -> Value {
    source_vectors()
        .into_iter()
        .find(|v| v["name"] == name)
        .unwrap()
}
pub fn bytes(value: &Value) -> Vec<u8> {
    let raw = value["hex"].as_str().unwrap().as_bytes();
    assert_eq!(raw.len() % 2, 0);
    raw.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
pub fn inconsistent<T>(result: Result<T, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}
pub fn reject_value(vector: &Value, projection: &Value) {
    inconsistent(procedural_fact_codec::values(
        vector["family"].as_str().unwrap(),
        &bytes(vector),
        projection,
    ));
}
pub fn reject_source(vector: &Value, projection: &Value) {
    inconsistent(procedural_fact_codec::sources(&bytes(vector), projection));
}
pub fn object_paths(value: &Value) -> Vec<String> {
    fn visit(value: &Value, path: String, paths: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                paths.push(path.clone());
                for (key, child) in object {
                    visit(child, format!("{path}/{key}"), paths);
                }
            }
            Value::Array(array) => {
                for (index, child) in array.iter().enumerate() {
                    visit(child, format!("{path}/{index}"), paths);
                }
            }
            _ => {}
        }
    }
    let mut paths = vec![];
    visit(value, String::new(), &mut paths);
    paths
}
pub fn reject_changed_values(name: &str, path: &str, changes: Vec<Value>) {
    let vector = value_vector(name);
    for changed in changes {
        let mut projection = vector["normalized"].clone();
        *projection.pointer_mut(path).unwrap() = changed;
        reject_value(&vector, &projection);
    }
}
pub fn reject_changed_sources(name: &str, path: &str, changes: Vec<Value>) {
    let vector = source_vector(name);
    for changed in changes {
        let mut projection = vector["sources"].clone();
        *projection.pointer_mut(path).unwrap() = changed;
        reject_source(&vector, &projection);
    }
}

#[test]
fn canonical_bounds_are_checked_even_for_nonobject_projections() {
    for (family, minimum, maximum) in [("resolution", 27, 17700), ("notification", 67, 58671)] {
        inconsistent(procedural_fact_codec::values(
            family,
            &vec![0; minimum - 1],
            &Value::Null,
        ));
        inconsistent(procedural_fact_codec::values(
            family,
            &vec![0; maximum + 1],
            &Value::Null,
        ));
    }
    inconsistent(procedural_fact_codec::sources(&[0; 18], &Value::Null));
    inconsistent(procedural_fact_codec::sources(
        &vec![0; 36848],
        &Value::Null,
    ));
}
