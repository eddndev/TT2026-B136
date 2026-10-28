//! End-to-end validation of complete and altered evidence packages built from
//! the fixtures, against the contract in
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod support;

mod admission;
mod hostile;
mod individual;
mod individual_negative;
mod server_seal;
mod server_seal_negative;
mod trust;
