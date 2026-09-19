//! Explicit reversible projections checked against their normalized values.
mod primitives;
mod values;
use application::{procedural_resources::ProceduralResourceError, ApplicationError};
pub(crate) use primitives::{decode_supports, encode_supports};
pub(crate) use values::{decode_act, decode_values, encode_act, encode_values};
type Result<T> = std::result::Result<T, ApplicationError>;
fn invalid() -> ApplicationError {
    ProceduralResourceError::StoredInconsistent("resource projection is not canonical".into())
        .into()
}
