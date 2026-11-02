use super::*;
use crate::contracts::test_support::request;

const BINDING_1: &str = "-signature parte-1-vinculo.sig parte-1-vinculo.bin";
const BINDING_2: &str = "-signature parte-2-vinculo.sig parte-2-vinculo.bin";
const CERTIFICATE_1: &str =
    "openssl verify -CAfile ca.pem -crl_check -CRLfile parte-1-crl.pem parte-1-certificado.pem";
const CERTIFICATE_2: &str =
    "openssl verify -CAfile ca.pem -crl_check -CRLfile parte-2-crl.pem parte-2-certificado.pem";
const CHAIN: [&str; 3] = [
    "dd if=parte-2-declaracion.bin bs=1 skip=239 count=32 | xxd -p -c 32",
    "dd if=parte-2-declaracion.bin bs=1 skip=271 count=32 | xxd -p -c 32",
    "sha256sum parte-1-declaracion.sig parte-1-declaracion.sig.tsr",
];

#[test]
fn the_partial_instructions_check_the_client_binding_and_certificate() {
    let text = instructions(&request(100, 0), ContractStage::Partial);
    assert!(text.contains(BINDING_1), "{text}");
    assert!(text.contains(CERTIFICATE_1), "{text}");
    assert!(!text.contains(BINDING_2) && !text.contains(CERTIFICATE_2));
    assert!(CHAIN.iter().all(|command| !text.contains(command)));
}

#[test]
fn the_full_instructions_check_both_parties_and_the_counter_signature_chain() {
    let text = instructions(&request(100, 0), ContractStage::Final);
    for command in [BINDING_1, BINDING_2, CERTIFICATE_1, CERTIFICATE_2] {
        assert!(text.contains(command), "{command}");
    }
    for command in CHAIN {
        assert!(text.contains(command), "{command}");
    }
    assert!(!text.contains("{{"), "{text}");
}
