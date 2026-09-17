#![allow(dead_code)]
use crate::case_administration_support::Fixture;
use application::ApplicationError;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;
use uuid::Uuid;

pub fn open(db: &Fixture) -> Result<PostgresCaseRepository, ApplicationError> {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
}
pub fn receipt(
    actor: Uuid,
    case: Uuid,
    id: Uuid,
    operation: Uuid,
    action: u8,
    expected: u32,
    reason: Option<&str>,
) -> Vec<u8> {
    let mut bytes = b"DLTX1".to_vec();
    for value in [actor, case, id, operation] {
        bytes.extend(value.as_bytes());
    }
    bytes.push(action);
    bytes.extend(expected.to_be_bytes());
    bytes.extend([23; 32]);
    bytes.push(u8::from(reason.is_some()));
    if let Some(reason) = reason {
        bytes.extend((reason.len() as u64).to_be_bytes());
        bytes.extend(reason.as_bytes());
    }
    bytes
}
