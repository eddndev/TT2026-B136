//! Known-answer tests for the AES-256-GCM cipher adapter.
//!
//! Vectors: test cases 13 through 16 for AES-256-GCM from "The
//! Galois/Counter Mode of Operation (GCM)" by McGrew and Viega, the
//! algorithm specification submitted to NIST that underlies NIST SP
//! 800-38D. The same values appear in the NIST CAVP GCM known-answer
//! suites (gcmEncryptExtIV256 / gcmDecrypt256).

use domain::crypto::cipher::{AuthenticatedCipher, SealedPayload, AES_GCM_NONCE_LEN};
use domain::DomainError;
use infrastructure::RingAesGcmCipher;

/// Decodes a lower-case hex string; test vectors are given in hex.
fn hex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "hex strings have even length");
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).expect("valid hex"))
        .collect()
}

struct Vector {
    key: &'static str,
    iv: &'static str,
    plaintext: &'static str,
    aad: &'static str,
    ciphertext: &'static str,
    tag: &'static str,
}

/// Test case 13: zero key and IV, empty plaintext, empty AAD.
const TC13: Vector = Vector {
    key: "0000000000000000000000000000000000000000000000000000000000000000",
    iv: "000000000000000000000000",
    plaintext: "",
    aad: "",
    ciphertext: "",
    tag: "530f8afbc74536b9a963b4f1c4cb738b",
};

/// Test case 14: zero key and IV, one block of zero plaintext, empty AAD.
const TC14: Vector = Vector {
    key: "0000000000000000000000000000000000000000000000000000000000000000",
    iv: "000000000000000000000000",
    plaintext: "00000000000000000000000000000000",
    aad: "",
    ciphertext: "cea7403d4d606b6e074ec5d3baf39d18",
    tag: "d0d1c8a799996bf0265b98b5d48ab919",
};

/// Test case 15: four blocks of plaintext, empty AAD.
const TC15: Vector = Vector {
    key: "feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308",
    iv: "cafebabefacedbaddecaf888",
    plaintext: "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
                1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255",
    aad: "",
    ciphertext: "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
                 8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad",
    tag: "b094dac5d93471bdec1a502270e3cc6c",
};

/// Test case 16: 60 bytes of plaintext with 20 bytes of AAD.
const TC16: Vector = Vector {
    key: "feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308",
    iv: "cafebabefacedbaddecaf888",
    plaintext: "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
                1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    aad: "feedfacedeadbeeffeedfacedeadbeefabaddad2",
    ciphertext: "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
                 8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662",
    tag: "76fc6ece0f4e1768cddf8853bb2d551b",
};

fn nonce_of(vector: &Vector) -> [u8; AES_GCM_NONCE_LEN] {
    hex(vector.iv).try_into().expect("96-bit IV")
}

fn sealed_bytes_of(vector: &Vector) -> Vec<u8> {
    let mut bytes = hex(vector.iv);
    bytes.extend_from_slice(&hex(vector.ciphertext));
    bytes.extend_from_slice(&hex(vector.tag));
    bytes
}

fn assert_encrypts(vector: &Vector) {
    let cipher = RingAesGcmCipher::new();
    let payload = cipher
        .seal_with_nonce(
            &hex(vector.key),
            &nonce_of(vector),
            &hex(vector.aad),
            &hex(vector.plaintext),
        )
        .expect("sealing succeeds");
    assert_eq!(payload.as_bytes(), sealed_bytes_of(vector).as_slice());
}

fn assert_decrypts(vector: &Vector) {
    let cipher = RingAesGcmCipher::new();
    let payload = SealedPayload::from_bytes(sealed_bytes_of(vector)).expect("valid layout");
    let plaintext = cipher
        .open(&hex(vector.key), &hex(vector.aad), &payload)
        .expect("authentic payload opens");
    assert_eq!(plaintext, hex(vector.plaintext));
}

#[test]
fn encrypt_vector_tc13_empty_plaintext() {
    assert_encrypts(&TC13);
}

#[test]
fn encrypt_vector_tc14_single_zero_block() {
    assert_encrypts(&TC14);
}

#[test]
fn encrypt_vector_tc15_four_blocks() {
    assert_encrypts(&TC15);
}

#[test]
fn encrypt_vector_tc16_with_aad() {
    assert_encrypts(&TC16);
}

#[test]
fn decrypt_vector_tc14_authenticates() {
    assert_decrypts(&TC14);
}

#[test]
fn decrypt_vector_tc16_authenticates() {
    assert_decrypts(&TC16);
}

#[test]
fn decrypt_vector_tc14_with_wrong_tag_fails() {
    let cipher = RingAesGcmCipher::new();
    let mut bytes = sealed_bytes_of(&TC14);
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    let payload = SealedPayload::from_bytes(bytes).expect("valid layout");
    let err = cipher
        .open(&hex(TC14.key), &hex(TC14.aad), &payload)
        .unwrap_err();
    assert_eq!(err, DomainError::AuthenticationFailed);
}
