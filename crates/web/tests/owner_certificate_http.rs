#[path = "owner_certificate_http_cases/body.rs"]
mod body;
#[allow(dead_code)]
#[path = "../../application/tests/case_support/mod.rs"]
mod case_support;
#[path = "owner_certificate_http_cases/current.rs"]
mod current;
#[path = "owner_certificate_http_cases/errors.rs"]
mod errors;
mod owner_certificate_http_support;
#[path = "owner_certificate_http_cases/routes.rs"]
mod routes;
#[allow(dead_code)]
#[path = "../../application/tests/owner_certificate_cases/submission_support.rs"]
mod submission_support;
#[allow(dead_code)]
#[path = "../../application/tests/owner_certificate_cases/support.rs"]
mod support;
