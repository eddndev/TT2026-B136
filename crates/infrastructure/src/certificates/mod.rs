//! Adapters for the internal public key infrastructure: X.509 chain
//! validation and the OpenSSL-script-backed certificate authority.
//!
//! The trust model is a single-layer chain (root authority to end-entity
//! certificate), matching what the scripts under `pki/` produce. Signature
//! verification re-encodes parsed to-be-signed structures and checks them
//! with the rsa crate; the approach and its trade-offs are recorded in
//! docs/adr/0004-certificate-validation-implementation.md.

mod authority;
mod document_signing;
mod internal_declaration;
mod internal_profile;
mod owner_binding;
mod owner_login;
mod parse;
mod request_authority;
mod request_material;
mod request_profile;
mod validator;

pub use authority::OpensslCaAdapter;
pub use document_signing::{DocumentSigningFailure, InternalRsaDocumentSigningVerifier};
pub use internal_declaration::InternalRsaDeclarationVerifier;
pub use internal_profile::ca_inspection::{
    inspect_ca_certificate, inspect_ca_request, CaCertificateInspection, CaRequestInspection,
};
pub use internal_profile::ca_inventory::{
    inspect_ca_inventory_certificate, CaInventoryCertificateInspection, CaInventoryPurpose,
};
pub use owner_binding::{InternalRsaOwnerBindingVerifier, OwnerBindingFailure};
pub use owner_login::{InternalRsaOwnerLoginVerifier, OwnerLoginFailure};
pub use request_profile::InternalRsaCertificateRequestProfileValidator;
pub use validator::X509ChainValidator;
