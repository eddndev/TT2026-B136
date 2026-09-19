//! Coverage of the Argon2id password hashing adapter beyond its round
//! trip: the default construction and the rejection of a stored hash this
//! adapter cannot recompute.

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher as _, Version};
use domain::crypto::password::{PasswordHasher, PasswordVerification};
use domain::DomainError;
use infrastructure::Argon2idHasher;

#[test]
fn the_default_hasher_produces_the_fixed_argon2id_parameters() {
    let phc = Argon2idHasher::default().hash("correct horse").unwrap();
    assert!(phc.starts_with("$argon2id$v=19$"), "got: {phc}");
    assert!(phc.contains("m=262144,t=3,p=1"), "got: {phc}");
}

#[test]
fn historical_two_pass_hashes_still_verify_with_the_current_hasher() {
    let salt = SaltString::encode_b64(b"historical-salt!").unwrap();
    let historical = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(262144, 2, 1, Some(32)).unwrap(),
    );
    let phc = historical
        .hash_password(b"historical password", &salt)
        .unwrap()
        .to_string();
    assert!(phc.contains("m=262144,t=2,p=1"), "got: {phc}");
    let hasher = Argon2idHasher::new();
    assert_eq!(
        hasher.verify("historical password", &phc).unwrap(),
        PasswordVerification::Match
    );
    assert_eq!(
        hasher.verify("wrong password", &phc).unwrap(),
        PasswordVerification::Mismatch
    );
}

#[test]
fn a_stored_hash_with_out_of_range_parameters_is_malformed_not_a_mismatch() {
    let hasher = Argon2idHasher::new();
    // A real hash whose memory cost is edited below the algorithm's minimum
    // still parses as a PHC string but cannot be recomputed, so it is a
    // malformed stored hash rather than a plain password mismatch.
    let phc = hasher.hash("password").unwrap().replace("m=262144", "m=1");
    assert_eq!(
        hasher.verify("password", &phc).unwrap_err(),
        DomainError::MalformedPasswordHash
    );
}
