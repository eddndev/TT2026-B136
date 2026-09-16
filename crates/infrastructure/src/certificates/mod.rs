//! Adapters for the internal public key infrastructure: X.509 chain
//! validation and the OpenSSL-script-backed certificate authority.
//!
//! The trust model is a single-layer chain (root authority to end-entity
//! certificate), matching what the scripts under `pki/` produce. Signature
//! verification re-encodes parsed to-be-signed structures and checks them
//! with the rsa crate; the approach and its trade-offs are recorded in
//! docs/adr/0004-certificate-validation-implementation.md.

mod authority;
mod internal_declaration;
mod parse;
mod validator;

pub use authority::OpensslCaAdapter;
pub use internal_declaration::InternalRsaDeclarationVerifier;
pub use validator::X509ChainValidator;
