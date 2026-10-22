//! Equivalence of catalog validators over generated catalog mutations.
//!
//! For every mutation of a module corpus, each validator runs through a direct
//! client and through `PreparedCatalog`; both verdicts must be identical in
//! variant and message. See
//! docs/adr/0083-call-scoped-prepared-catalog-statements.md.

use super::catalog_client::PreparedCatalog;
use super::mutation_corpus::{mutations, ModuleSpec};
use super::test_fixture::CatalogFixture;
use super::CatalogClient;
use application::ApplicationError;

mod alert;

/// One validator of a module, runnable through any catalog client.
pub(super) trait Check: Copy + std::fmt::Debug {
    fn run<C: CatalogClient>(self, client: &mut C, role: &str) -> Result<(), ApplicationError>;
}

/// Runs every check of a module on the unaltered catalog and on each
/// generated mutation. Returns the names of the cases that every check
/// accepted, so the caller can pin which mutations its validators tolerate.
pub(super) fn legacy_verdicts<K: Check>(
    family: &str,
    spec: &ModuleSpec,
    checks: &[K],
) -> Option<Vec<String>> {
    let mut db = CatalogFixture::new()?;
    let role = db.role.clone();
    let cases = std::iter::once(("unaltered".to_owned(), String::new()))
        .chain(mutations(spec).into_iter().map(|m| (m.name, m.sql)));
    let mut accepted = Vec::new();
    for (name, sql) in cases {
        let mut state = db
            .mutated(&sql)
            .unwrap_or_else(|error| panic!("cannot apply {name}: {error:?}"));
        let mut rejected = false;
        for &check in checks {
            let legacy = state.run(|client| check.run(client, &role));
            let prepared = state.run(|client| check.run(&mut PreparedCatalog::new(client), &role));
            let (direct, through) = (format!("{legacy:?}"), format!("{prepared:?}"));
            assert_eq!(
                direct, through,
                "{name}: {family} {check:?} diverged through PreparedCatalog"
            );
            rejected |= legacy.is_err();
        }
        if !rejected {
            accepted.push(name);
        }
    }
    Some(accepted)
}
