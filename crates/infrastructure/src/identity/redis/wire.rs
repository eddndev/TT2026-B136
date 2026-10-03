use std::{fmt, marker::PhantomData};

use application::identity::{Principal, SessionIdentity};
use domain::identity::{Role, UserId};
use serde::{
    de::{value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

pub(super) const MAX_EXACT: i64 = 9_007_199_254_740_991;

// Struct deserialization normally also accepts arrays. Stored identities and
// every nested certificate object must instead use their named-field format.
pub(super) fn object<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Object<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Object<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object with exact named fields")
        }

        fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<T, M::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_map(Object(PhantomData))
}

pub(super) fn parse<'de, T: Deserialize<'de>>(raw: &'de str) -> Option<T> {
    let mut parser = serde_json::Deserializer::from_str(raw);
    let value = object(&mut parser).ok()?;
    parser.end().ok()?;
    Some(value)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredPrincipal {
    pub id: UserId,
    pub email: String,
    #[serde(deserialize_with = "role")]
    pub role: Role,
}

fn role<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Role, D::Error> {
    match String::deserialize(deserializer)?.as_str() {
        "owner" => Ok(Role::Owner),
        "litigator" => Ok(Role::Litigator),
        "paralegal" => Ok(Role::Paralegal),
        "client" => Ok(Role::Client),
        _ => Err(serde::de::Error::custom("invalid stored role")),
    }
}

impl From<Principal> for StoredPrincipal {
    fn from(value: Principal) -> Self {
        Self {
            id: value.id,
            email: value.email,
            role: value.role,
        }
    }
}

impl From<StoredPrincipal> for Principal {
    fn from(value: StoredPrincipal) -> Self {
        Self {
            id: value.id,
            email: value.email,
            role: value.role,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSession {
    #[serde(deserialize_with = "object")]
    principal: StoredPrincipal,
    auth_generation: u64,
}

pub(super) fn identity(raw: &str) -> Option<SessionIdentity> {
    let value: StoredSession = parse(raw)?;
    if value.auth_generation > i64::MAX as u64 {
        return None;
    }
    Some(SessionIdentity {
        principal: value.principal.into(),
        auth_generation: value.auth_generation,
    })
}

pub(super) fn milliseconds(seconds: i64) -> Option<i64> {
    seconds
        .checked_mul(1000)
        .filter(|value| (0..=MAX_EXACT).contains(value))
}
