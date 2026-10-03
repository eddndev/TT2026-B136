//! Self-Owner certificate registration and terminal withdrawal, without login.
//!
//! Public evidence is prepared outside the store transaction. The store must
//! reauthorize and revalidate it under its audit and mutation locks before any
//! mutation. See docs/adr/0067-owner-certificate-bindings.md.

mod evidence;
mod model;
mod port;
mod prepared;
mod registration;
mod service;
mod withdrawal;

pub use model::{
    OwnerBindingAccount, OwnerBindingCommit, OwnerBindingReceipt, OwnerCertificateError,
    OwnerRegistrationContext, OwnerWithdrawalContext,
};
pub use port::{OwnerBindingVerifier, OwnerCertificatePorts, OwnerCertificateStore};
pub use prepared::{PreparedOwnerRegistration, PreparedOwnerWithdrawal, VerifiedOwnerRegistration};
pub use service::OwnerCertificateService;
