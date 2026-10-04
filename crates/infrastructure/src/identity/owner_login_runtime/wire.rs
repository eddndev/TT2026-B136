use serde::{Deserialize, Serialize};

use super::codec::object;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope {
    pub version: u8,
    #[serde(deserialize_with = "object")]
    pub context: Context,
    pub statement: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    #[serde(deserialize_with = "object")]
    pub account: Account,
    pub binding_id: String,
    #[serde(deserialize_with = "object")]
    pub certificate: Certificate,
    #[serde(deserialize_with = "object")]
    pub trust: Trust,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Account {
    #[serde(deserialize_with = "object")]
    pub principal: Principal,
    pub active: bool,
    pub revision: u64,
    pub auth_generation: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Principal {
    pub id: String,
    pub email: String,
    pub role: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Certificate {
    pub der: String,
    pub fingerprint: String,
    #[serde(deserialize_with = "object")]
    pub summary: Summary,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Summary {
    pub subject: String,
    pub issuer: String,
    pub serial_hex: String,
    pub not_before_unix: i64,
    pub not_after_unix: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Trust {
    pub deployment_id: String,
    pub revision: u32,
    #[serde(deserialize_with = "object")]
    pub inspection: Inspection,
    #[serde(deserialize_with = "object")]
    pub published_at: Publication,
    pub published_by: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inspection {
    pub root_der: String,
    pub crl_der: String,
    pub root_fingerprint: String,
    pub crl_digest: String,
    pub crl_number: u64,
    pub crl_this_update: i64,
    pub crl_next_update: i64,
    pub valid_from: i64,
    pub valid_until: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Publication {
    pub seconds: i64,
    pub nanoseconds: u32,
}
