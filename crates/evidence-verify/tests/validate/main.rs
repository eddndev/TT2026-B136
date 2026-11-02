//! End-to-end validation of complete and altered evidence packages built from
//! the fixtures, against the contracts in
//! `docs/adr/0085-in-browser-evidence-validation.md` and
//! `docs/adr/0086-bilateral-contract-signing.md`.

mod contract_support;
mod support;

mod admission;
mod contract;
mod contract_negative;
mod hostile;
mod individual;
mod individual_negative;
mod individual_v2;
mod individual_v2_edges;
mod individual_v2_negative;
mod server_seal;
mod server_seal_negative;
mod trust;
