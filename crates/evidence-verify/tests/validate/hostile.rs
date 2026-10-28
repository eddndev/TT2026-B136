//! Hostile entry contents: byte flips, truncations and insertions in every
//! entry of complete packages, with the archive rebuilt so that admission
//! passes and, for individual signing, the record rewritten so that it
//! still matches the altered statement entries.
//! Validation must return a report for each, never panic, and never call
//! an altered package complete unless the alteration leaves what the
//! validator reads unchanged.

use evidence_verify::policy::DEFAULT_ADMITTED_POLICY;
use evidence_verify::timestamp::{self, TokenFacts};
use evidence_verify::{certificate, individual_json, pem, validate, Outcome};

use crate::support::{fixture, policy, Package};

/// A small deterministic generator, so that every run probes the same
/// inputs.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next()).expect("fits") % bound.max(1)
    }
}

fn probe(base: &Package, rounds: usize, seed: u64, rewrite_record: bool) {
    let policy = policy();
    let mut lcg = Lcg(seed);
    for round in 0..rounds {
        let mut package = base.clone();
        let index = lcg.below(package.entries.len());
        let (name, data) = package.entries[index].clone();
        let mut mutated = data.clone();
        match round % 3 {
            0 if !mutated.is_empty() => {
                let at = lcg.below(mutated.len());
                mutated[at] ^= 1 << lcg.below(8);
            }
            1 => mutated.truncate(lcg.below(mutated.len())),
            _ => {
                let at = lcg.below(mutated.len());
                let value = u8::try_from(lcg.below(256)).expect("byte");
                mutated.insert(at, value);
            }
        }
        if mutated == data {
            continue;
        }
        package.set(&name, &mutated);
        if rewrite_record && name != "individual-signing.json" {
            let (registered, accepted) = crate::individual::instants();
            crate::individual::rewrite_json(&mut package, registered, accepted);
        }
        let report = validate(&package.zip(), &policy);
        if report.overall == Outcome::Checked {
            assert!(
                same_meaning(&name, &data, &mutated),
                "{name} altered in round {round} is complete: {report:#?}"
            );
        }
    }
}

/// Whether two contents of `name` state the same thing to the validator:
/// the uninterpreted instructions, PEM entries that decode to the same DER,
/// a record that parses to the same values, or a token whose verified facts
/// agree except for the unsigned certificates it carries.
fn same_meaning(name: &str, before: &[u8], after: &[u8]) -> bool {
    if name == "INSTRUCCIONES.md" {
        return true;
    }
    if name.ends_with(".pem") {
        let label = if name.starts_with("crl") {
            "X509 CRL"
        } else {
            "CERTIFICATE"
        };
        return pem::decode_all(after, label, 8).ok() == pem::decode_all(before, label, 8).ok();
    }
    if name == "individual-signing.json" {
        return individual_json::parse(after).ok() == individual_json::parse(before).ok();
    }
    if name.ends_with(".tsr") {
        let covered = certificate::sha256(fixture!("document.bin"));
        let facts = |tsr| {
            timestamp::verify(tsr, &covered, &[DEFAULT_ADMITTED_POLICY]).map(|facts| TokenFacts {
                certificates: Vec::new(),
                ..facts
            })
        };
        return facts(after).ok() == facts(before).ok();
    }
    false
}

#[test]
fn altered_server_seal_entries_never_panic_or_pass_unnoticed() {
    probe(
        &Package::server_seal(fixture!("token-ess-v1.tsr")),
        ROUNDS,
        1,
        false,
    );
}

#[test]
fn altered_individual_entries_never_panic_or_pass_unnoticed() {
    probe(&crate::individual::complete(), ROUNDS, 2, true);
}

/// Rounds per package: enough to reach every entry several times with each
/// kind of alteration while keeping the suite fast.
const ROUNDS: usize = 100;
