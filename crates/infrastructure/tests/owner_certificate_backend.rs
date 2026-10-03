#[path = "owner_certificate_backend_cases/atomicity.rs"]
mod atomicity;
#[path = "owner_certificate_backend_cases/capture.rs"]
mod capture;
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "owner_certificate_backend_cases/concurrency.rs"]
mod concurrency;
#[allow(dead_code)]
#[path = "declaration_fixture/mod.rs"]
mod declaration_fixture;
#[path = "owner_certificate_backend_cases/history.rs"]
mod history;
#[path = "owner_certificate_backend_cases/inventory.rs"]
mod inventory;
#[path = "owner_certificate_backend_cases/lock_waits.rs"]
mod lock_waits;
#[path = "owner_certificate_backend_cases/locking.rs"]
mod locking;
#[allow(dead_code)]
mod owner_binding_fixture;
#[path = "owner_certificate_backend_cases/replication_permissions.rs"]
mod replication_permissions;
#[path = "owner_certificate_backend_cases/support.rs"]
mod support;
