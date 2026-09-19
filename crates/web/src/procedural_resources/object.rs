pub(super) use crate::procedural_facts::object::Object;
use serde::{Deserialize, Deserializer};
pub(super) fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Object<T>>, D::Error> {
    Option::<Object<T>>::deserialize(d)
}
