use domain::{
    crypto::Sha256Digest,
    identity::Role,
    owner_certificate_login::{LoginAccount, LoginError, LoginNonce, LoginStatement},
    owner_certificates::{BindingMaterial, BindingRecord, BindingStatement, OwnerAccount},
};
use uuid::Uuid;

mod owner_certificate_login_support;
use owner_certificate_login_support::*;

#[test]
fn login_matches_the_complete_independent_literal_vector() {
    let value = statement();
    let bytes = value.canonical_bytes();
    assert_eq!(bytes.len(), 182);
    assert_eq!(LITERAL.len(), 364);
    assert_eq!(hex(&bytes), LITERAL);
    assert_eq!(value.owner().id(), user(0x33));
    assert_eq!(value.owner().generation(), GENERATION);
    assert_eq!(value.material(), &material(0x0102_0304));
    assert_eq!(value.nonce().as_bytes(), &NONCE);
    assert_eq!(value.issued_at_unix_seconds(), ISSUED);
    assert_eq!(value.expires_at_unix_seconds(), EXPIRES);
}

#[test]
fn maximum_generation_trust_and_time_match_a_second_literal_vector() {
    let value = LoginStatement::new(
        LoginAccount::new(user(0x33), Role::Owner, true, i64::MAX as u64).unwrap(),
        material(u32::MAX),
        LoginNonce::from_bytes(&[0xff; 32]).unwrap(),
        i64::MAX - 300,
        i64::MAX,
    )
    .unwrap();
    assert_eq!(MAXIMUM_LITERAL.len(), 364);
    assert_eq!(hex(&value.canonical_bytes()), MAXIMUM_LITERAL);
    assert!(value.is_live_at(i64::MAX - 300));
    assert!(value.is_live_at(i64::MAX - 1));
    assert!(!value.is_live_at(i64::MAX));
}

#[test]
fn login_requires_an_active_non_nil_owner_and_a_persistable_generation() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        assert_eq!(
            LoginAccount::new(user(0x33), role, true, 0),
            Err(LoginError::OwnerRequired)
        );
    }
    assert_eq!(
        LoginAccount::new(user(0x33), Role::Owner, false, 0),
        Err(LoginError::OwnerRequired)
    );
    assert_eq!(
        LoginAccount::new(user(0), Role::Owner, true, 0),
        Err(LoginError::InvalidIdentity)
    );
    for generation in [i64::MAX as u64 + 1, u64::MAX] {
        assert_eq!(
            LoginAccount::new(user(0x33), Role::Owner, true, generation),
            Err(LoginError::InvalidGeneration)
        );
    }
    for generation in [0, 1, i64::MAX as u64] {
        let owner = LoginAccount::new(user(0x33), Role::Owner, true, generation).unwrap();
        assert_eq!(owner.generation(), generation);
    }
}

#[test]
fn nonce_has_exactly_32_owned_bytes_without_claiming_to_generate_entropy() {
    for length in [0, 1, 31, 33, 64] {
        assert_eq!(
            LoginNonce::from_bytes(&vec![0x66; length]),
            Err(LoginError::InvalidNonceLength)
        );
    }
    let mut input = NONCE;
    let nonce = LoginNonce::from_bytes(&input).unwrap();
    input.fill(0xff);
    assert_eq!(nonce.as_bytes(), &NONCE);
    assert_eq!(
        LoginNonce::from_bytes(&[0; 32]).unwrap().as_bytes(),
        &[0; 32]
    );
}

#[test]
fn validity_window_is_positive_bounded_and_does_not_overflow() {
    for (issued, expires) in [
        (-1, 1),
        (i64::MIN, 1),
        (0, 0),
        (0, -1),
        (1, 0),
        (0, 301),
        (ISSUED, ISSUED + 301),
        (0, i64::MAX),
        (i64::MIN, i64::MAX),
        (i64::MAX, i64::MAX),
    ] {
        assert_eq!(
            LoginStatement::new(
                account(),
                material(1),
                LoginNonce::from_bytes(&NONCE).unwrap(),
                issued,
                expires
            ),
            Err(LoginError::InvalidTimeWindow),
            "unexpectedly admitted window {issued}..{expires}"
        );
    }
    for (issued, expires) in [
        (0, 1),
        (0, 300),
        (ISSUED, EXPIRES),
        (i64::MAX - 1, i64::MAX),
    ] {
        assert!(LoginStatement::new(
            account(),
            material(1),
            LoginNonce::from_bytes(&NONCE).unwrap(),
            issued,
            expires
        )
        .is_ok());
    }
}

#[test]
fn validity_includes_issue_time_and_excludes_the_expiry_boundary() {
    let value = statement();
    for now in [i64::MIN, -1, ISSUED - 1, EXPIRES, EXPIRES + 1, i64::MAX] {
        assert!(!value.is_live_at(now), "unexpectedly live at {now}");
    }
    for now in [ISSUED, ISSUED + 1, EXPIRES - 1] {
        assert!(value.is_live_at(now), "unexpectedly expired at {now}");
    }
    assert_eq!(hex(&value.canonical_bytes()), LITERAL);
}

#[test]
fn every_account_material_nonce_and_time_field_changes_the_signed_bytes() {
    let original = statement().canonical_bytes();
    for field in 0..10 {
        let owner = LoginAccount::new(
            user(if field == 0 { 0x34 } else { 0x33 }),
            Role::Owner,
            true,
            if field == 1 {
                GENERATION + 1
            } else {
                GENERATION
            },
        )
        .unwrap();
        let material = BindingMaterial::new(
            Uuid::from_bytes([if field == 2 { 0x12 } else { 0x11 }; 16]),
            Uuid::from_bytes([if field == 3 { 0x45 } else { 0x44 }; 16]),
            Sha256Digest::from_array([if field == 4 { 0x23 } else { 0x22 }; 32]),
            Sha256Digest::from_array([if field == 5 { 0x56 } else { 0x55 }; 32]),
            if field == 6 { 0x0102_0305 } else { 0x0102_0304 },
        )
        .unwrap();
        let nonce = if field == 7 { [0x77; 32] } else { NONCE };
        let changed = LoginStatement::new(
            owner,
            material,
            LoginNonce::from_bytes(&nonce).unwrap(),
            if field == 8 { ISSUED + 1 } else { ISSUED },
            if field == 9 { EXPIRES - 1 } else { EXPIRES },
        )
        .unwrap();
        assert_ne!(changed.canonical_bytes(), original, "unbound field {field}");
    }
}

#[test]
fn login_is_separate_from_registration_and_terminal_withdrawal() {
    let owner = OwnerAccount::new(user(0x33), Role::Owner, true, GENERATION, GENERATION).unwrap();
    let registration = BindingStatement::new(owner, user(0x33), material(0x0102_0304)).unwrap();
    let record = BindingRecord::registered(registration.clone())
        .withdraw(owner, 1)
        .unwrap();
    let login = statement().canonical_bytes();
    assert_eq!(&login[..10], b"OWNAUTH1\x01\x01");
    for other in [
        registration.canonical_bytes(),
        record.withdrawal().unwrap().canonical_bytes(),
    ] {
        assert_eq!(&other[..8], b"OWNCERT1");
        assert_ne!(&login[..8], &other[..8]);
        assert_ne!(login.as_slice(), other.as_slice());
    }
}

#[test]
fn canonical_output_is_an_independent_value_not_mutable_statement_state() {
    let value = statement();
    let mut bytes = value.canonical_bytes();
    bytes[8] = 2;
    bytes[9] = 2;
    bytes[134] ^= 0xff;
    assert_ne!(hex(&bytes), LITERAL);
    assert_eq!(hex(&value.canonical_bytes()), LITERAL);
}
