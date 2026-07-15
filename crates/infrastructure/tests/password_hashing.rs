//! Coverage of the Argon2id password hashing adapter beyond its round
//! trip: the default construction and the rejection of a stored hash this
//! adapter cannot recompute.

use domain::crypto::password::PasswordHasher;
use domain::DomainError;
use infrastructure::Argon2idHasher;

#[test]
fn the_default_hasher_produces_the_fixed_argon2id_parameters() {
    let phc = Argon2idHasher::default().hash("correct horse").unwrap();
    assert!(phc.starts_with("$argon2id$v=19$"), "got: {phc}");
    assert!(phc.contains("m=47104,t=1,p=1"), "got: {phc}");
}

#[test]
fn a_stored_hash_with_out_of_range_parameters_is_malformed_not_a_mismatch() {
    let hasher = Argon2idHasher::new();
    // A real hash whose memory cost is edited below the algorithm's minimum
    // still parses as a PHC string but cannot be recomputed, so it is a
    // malformed stored hash rather than a plain password mismatch.
    let phc = hasher.hash("password").unwrap().replace("m=47104", "m=1");
    assert_eq!(
        hasher.verify("password", &phc).unwrap_err(),
        DomainError::MalformedPasswordHash
    );
}
