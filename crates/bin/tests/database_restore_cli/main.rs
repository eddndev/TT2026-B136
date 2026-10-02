//! Administrative restore commands exercised through the compiled binary.

#[path = "../../../infrastructure/tests/case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "../../../infrastructure/tests/password_reset_cases/password_reset_backend_support.rs"]
mod password_reset_backend_support;
#[allow(dead_code)]
#[path = "../../../infrastructure/tests/password_reset_cases/password_reset_restore_support.rs"]
mod password_reset_restore_support;

mod arguments;
mod check;
mod invalidation;
mod support;
