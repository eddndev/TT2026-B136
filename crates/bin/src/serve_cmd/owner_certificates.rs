//! Compose audited self-Owner certificate evidence over validated runtime state.

use std::sync::Arc;

use anyhow::Context;
use application::identity::{
    owner_certificates::{OwnerCertificatePorts, OwnerCertificateService},
    IdentityWorkflow,
};
use infrastructure::{
    certificates::InternalRsaOwnerBindingVerifier, PostgresConnectionSource,
    PostgresOwnerCertificateStore, RingSha256Hasher, SystemClock,
};

pub(super) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<Arc<OwnerCertificateService>> {
    let clock = Arc::new(SystemClock::new());
    let store = PostgresOwnerCertificateStore::open(database, clock.clone())
        .context("cannot open PostgreSQL Owner certificate store")?;
    Ok(Arc::new(OwnerCertificateService::new(
        OwnerCertificatePorts {
            identity,
            store: Arc::new(store),
            verifier: Arc::new(InternalRsaOwnerBindingVerifier::new()),
            hasher: Arc::new(RingSha256Hasher::new()),
            clock,
        },
    )))
}
