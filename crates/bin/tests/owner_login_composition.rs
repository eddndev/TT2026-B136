//! Explicit certificate login configuration and shared identity composition.

#[allow(dead_code)]
#[path = "../src/serve_args.rs"]
mod serve_args;
#[allow(dead_code)]
#[path = "../src/serve_identity_composition.rs"]
mod serve_identity_composition;
#[allow(dead_code)]
#[path = "../src/serve_owner_login_config.rs"]
mod serve_owner_login_config;

#[allow(dead_code, unused_imports)]
#[path = "../../application/tests/owner_certificate_login_cases/mod.rs"]
mod support;

#[path = "owner_login_composition_cases/configuration.rs"]
mod configuration;
#[path = "owner_login_composition_cases/identity.rs"]
mod identity;
#[path = "owner_login_composition_cases/options.rs"]
mod options;
#[path = "owner_login_composition_cases/startup.rs"]
mod startup;
