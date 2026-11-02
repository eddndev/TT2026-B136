//! A Client certificate revoked in the current CRL after the Client signed
//! blocks the counter-signature before any time-stamp is requested:
//! docs/adr/0086-bilateral-contract-signing.md. It revokes a certificate of
//! the fixture authority, so it runs in its own test binary.
#![allow(dead_code)]
mod case_administration_support;
mod contract_service_support;
mod contract_storage_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{
    contracts::*,
    credential_trust::{CredentialTrustExpectation, CredentialTrustRevision, CredentialTrustStore},
};
use contract_service_support::{contract_error, Live};
use declaration_fixture::{fixture, openssl, script};
use domain::{clock::Clock as _, crypto::InternalDeclarationVerifier as _};
use infrastructure::{certificates::InternalRsaDeclarationVerifier, PostgresCredentialTrustStore};
use uuid::Uuid;

#[test]
fn a_revoked_client_certificate_blocks_the_counter_signature() {
    let Some(live) = Live::new() else { return };
    let mut live = live.requested();
    let (client, firm) = (live.client(), live.firm());
    live.sign(&client, &live.signed(&client, Uuid::new_v4()))
        .unwrap();

    let ca = &fixture().ca;
    let certificate = ca.join("certs/second-declarant.crt.pem");
    script("revoke.sh", ca, &[certificate.to_str().unwrap()]);
    script("gen-crl.sh", ca, &[]);
    let crl_path = ca.join("crl/crl.pem");
    let crl = openssl(&["crl", "-in", crl_path.to_str().unwrap(), "-outform", "DER"]);
    let root = individual_signing_fixture::trust().inspection.root_der;
    let at = live.clock.now().unix_timestamp() + 1;
    live.clock.shift(1);
    let inspection = InternalRsaDeclarationVerifier
        .inspect_trust(&root, &crl, at)
        .unwrap();
    let current: i64 = live
        .test
        .db()
        .admin
        .query_one(
            "SELECT max(revision) FROM participant_credential_trust_revisions",
            &[],
        )
        .unwrap()
        .get(0);
    PostgresCredentialTrustStore::open(&live.test.individual.db.admin_url, live.clock.clone())
        .unwrap()
        .publish(
            CredentialTrustExpectation::Revision(
                CredentialTrustRevision::new(u32::try_from(current).unwrap()).unwrap(),
            ),
            inspection,
        )
        .unwrap();

    let events = live.count("contract_events");
    let submitted = live.signed(&firm, Uuid::new_v4());
    assert_eq!(
        contract_error(live.sign(&firm, &submitted)),
        ContractWorkflowError::CounterpartyRevoked
    );
    assert_eq!(live.count("contract_events"), events);
    assert_eq!(live.count("contract_completions"), 0);
}
