use application::documents::EvidenceMaterial;
use base64::{engine::general_purpose::STANDARD, Engine};

use super::material;

fn pem(label: &str, der: &[u8]) -> Vec<u8> {
    let encoded = STANDARD.encode(der);
    let mut text = format!("-----BEGIN {label}-----\n");
    for line in encoded.as_bytes().chunks(64) {
        text.push_str(std::str::from_utf8(line).unwrap());
        text.push('\n');
    }
    text.push_str(&format!("-----END {label}-----\n"));
    text.into_bytes()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use domain::crypto::DocumentHasher;
    infrastructure::RingSha256Hasher::new()
        .hash_bytes(bytes)
        .to_hex()
}

fn evidence(chain: Option<Vec<u8>>) -> EvidenceMaterial {
    EvidenceMaterial {
        signer_certificate_pem: pem("CERTIFICATE", b"signer"),
        issuer_certificate_pem: pem("CERTIFICATE", b"seal root"),
        crl_pem: pem("X509 CRL", b"seal crl"),
        tsa_chain_pem: chain,
        openssl_version: "OpenSSL 3".into(),
    }
}

#[test]
fn a_two_certificate_chain_gives_two_time_stamping_roots_in_file_order() {
    let mut chain = pem("CERTIFICATE", b"tsa leaf");
    chain.extend(pem("CERTIFICATE", &[0x5a; 100]));

    let served = material(&evidence(Some(chain))).unwrap();

    assert_eq!(
        served.time_stamping,
        vec![
            (b"tsa leaf".to_vec(), sha256_hex(b"tsa leaf")),
            (vec![0x5a; 100], sha256_hex(&[0x5a; 100])),
        ]
    );
}

#[test]
fn every_fingerprint_is_the_lowercase_sha256_of_the_der() {
    let served = material(&evidence(Some(pem("CERTIFICATE", b"tsa")))).unwrap();

    assert_eq!(served.seal_root_der, b"seal root");
    assert_eq!(served.seal_root_fingerprint, sha256_hex(b"seal root"));
    assert_eq!(served.seal_crl_der, b"seal crl");
    assert_eq!(served.seal_crl_digest, sha256_hex(b"seal crl"));
    for value in [&served.seal_root_fingerprint, &served.time_stamping[0].1] {
        assert_eq!(value.len(), 64);
        assert!(value
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase()));
    }
}

#[test]
fn malformed_or_missing_material_is_an_error() {
    let tsa = || Some(pem("CERTIFICATE", b"tsa"));
    let mut two_roots = evidence(tsa());
    two_roots
        .issuer_certificate_pem
        .extend(pem("CERTIFICATE", b"other root"));
    let mut wrong_crl_label = evidence(tsa());
    wrong_crl_label.crl_pem = pem("CERTIFICATE", b"seal crl");
    let mut text_outside = evidence(tsa());
    text_outside.issuer_certificate_pem = b"not pem".to_vec();
    let bad_chain = evidence(Some(b"-----BEGIN CERTIFICATE-----\n!!\n".to_vec()));
    let missing_chain = evidence(None);

    for (name, value) in [
        ("two seal roots", two_roots),
        ("crl label", wrong_crl_label),
        ("text outside", text_outside),
        ("bad chain", bad_chain),
        ("missing chain", missing_chain),
    ] {
        assert!(material(&value).is_err(), "{name}");
    }
}
