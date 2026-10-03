use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
    owner_certificates::{
        BindingError, BindingMaterial, BindingRecord, BindingStatement, OwnerAccount,
    },
};
use uuid::Uuid;

const REVISION: u64 = 0x1112_1314_1516_1718;
const GENERATION: u64 = 0x0102_0304_0506_0708;

fn user(byte: u8) -> UserId {
    UserId::from_uuid(Uuid::from_bytes([byte; 16]))
}

fn owner(revision: u64, generation: u64) -> OwnerAccount {
    OwnerAccount::new(user(0x33), Role::Owner, true, revision, generation).unwrap()
}

fn material() -> BindingMaterial {
    BindingMaterial::new(
        Uuid::from_bytes([0x11; 16]),
        Uuid::from_bytes([0x44; 16]),
        Sha256Digest::from_array([0x22; 32]),
        Sha256Digest::from_array([0x55; 32]),
        0x0102_0304,
    )
    .unwrap()
}

fn statement() -> BindingStatement {
    BindingStatement::new(owner(REVISION, GENERATION), user(0x33), material()).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

#[test]
fn registration_matches_the_complete_literal_canonical_vector() {
    let actual = statement().canonical_bytes();
    assert_eq!(actual.len(), 150);
    assert_eq!(
        hex(&actual),
        concat!(
            "4f574e43455254310101",
            "11111111111111111111111111111111",
            "2222222222222222222222222222222222222222222222222222222222222222",
            "01020304",
            "33333333333333333333333333333333",
            "1112131415161718",
            "0102030405060708",
            "44444444444444444444444444444444",
            "00000000",
            "00000001",
            "5555555555555555555555555555555555555555555555555555555555555555"
        )
    );
}

#[test]
fn withdrawal_has_a_distinct_purpose_and_preserves_original_registration() {
    let original = statement();
    let registered = BindingRecord::registered(original.clone());
    assert_eq!(registered.revision(), 1);
    assert!(registered.withdrawal().is_none());
    let withdrawn = registered
        .withdraw(owner(REVISION + 1, GENERATION + 1), 1)
        .unwrap();
    assert_eq!(withdrawn.revision(), 2);
    assert_eq!(withdrawn.registration(), &original);
    let actual = withdrawn.withdrawal().unwrap().canonical_bytes();
    assert_eq!(actual.len(), 150);
    assert_ne!(actual, original.canonical_bytes());
    assert_eq!(
        hex(&actual),
        concat!(
            "4f574e43455254310201",
            "11111111111111111111111111111111",
            "2222222222222222222222222222222222222222222222222222222222222222",
            "01020304",
            "33333333333333333333333333333333",
            "1112131415161719",
            "0102030405060709",
            "44444444444444444444444444444444",
            "00000001",
            "00000002",
            "5555555555555555555555555555555555555555555555555555555555555555"
        )
    );
}

#[test]
fn owner_authority_requires_an_active_account_and_the_same_target() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        assert_eq!(
            OwnerAccount::new(user(0x33), role, true, 0, 0),
            Err(BindingError::OwnerRequired)
        );
    }
    assert_eq!(
        OwnerAccount::new(user(0x33), Role::Owner, false, 0, 0),
        Err(BindingError::OwnerRequired)
    );
    assert_eq!(
        BindingStatement::new(owner(0, 0), user(0x66), material()),
        Err(BindingError::DifferentOwner)
    );
    let other = OwnerAccount::new(user(0x66), Role::Owner, true, REVISION, GENERATION).unwrap();
    assert_eq!(
        BindingRecord::registered(statement()).withdraw(other, 1),
        Err(BindingError::DifferentOwner)
    );
}

#[test]
fn nil_identities_and_absent_trust_cannot_describe_a_binding() {
    assert_eq!(
        OwnerAccount::new(user(0), Role::Owner, true, 0, 0),
        Err(BindingError::InvalidIdentity)
    );
    let deployment = Uuid::from_bytes([0x11; 16]);
    let binding = Uuid::from_bytes([0x44; 16]);
    for (deployment, binding) in [(Uuid::nil(), binding), (deployment, Uuid::nil())] {
        assert_eq!(
            BindingMaterial::new(
                deployment,
                binding,
                Sha256Digest::from_array([0x22; 32]),
                Sha256Digest::from_array([0x55; 32]),
                1,
            ),
            Err(BindingError::InvalidIdentity)
        );
    }
    assert_eq!(
        BindingMaterial::new(
            deployment,
            binding,
            Sha256Digest::from_array([0x22; 32]),
            Sha256Digest::from_array([0x55; 32]),
            0,
        ),
        Err(BindingError::InvalidTrustRevision)
    );
}

#[test]
fn account_counters_obey_persisted_ranges_without_incrementing_them() {
    let maximum = i64::MAX as u64;
    for (revision, generation) in [(0, 0), (maximum, 0), (maximum, maximum)] {
        let input =
            BindingStatement::new(owner(revision, generation), user(0x33), material()).unwrap();
        assert_eq!(input.canonical_bytes().len(), 150);
        assert!(BindingRecord::registered(input)
            .withdraw(owner(revision, generation), 1)
            .is_ok());
    }
    for (revision, generation) in [
        (0, 1),
        (maximum + 1, 0),
        (u64::MAX, 0),
        (maximum, maximum + 1),
        (u64::MAX, u64::MAX),
    ] {
        assert_eq!(
            OwnerAccount::new(user(0x33), Role::Owner, true, revision, generation),
            Err(BindingError::InvalidAccountVersion)
        );
    }
    assert!(BindingMaterial::new(
        Uuid::from_u128(1),
        Uuid::from_u128(2),
        Sha256Digest::from_array([0; 32]),
        Sha256Digest::from_array([0; 32]),
        u32::MAX,
    )
    .is_ok());
}

#[test]
fn every_material_identity_and_fingerprint_changes_the_registered_bytes() {
    let initial = statement().canonical_bytes();
    for field in 0..5 {
        let changed = BindingMaterial::new(
            Uuid::from_bytes([if field == 0 { 0x12 } else { 0x11 }; 16]),
            Uuid::from_bytes([if field == 1 { 0x45 } else { 0x44 }; 16]),
            Sha256Digest::from_array([if field == 2 { 0x23 } else { 0x22 }; 32]),
            Sha256Digest::from_array([if field == 3 { 0x56 } else { 0x55 }; 32]),
            if field == 4 { 0x0102_0305 } else { 0x0102_0304 },
        )
        .unwrap();
        let bytes = BindingStatement::new(owner(REVISION, GENERATION), user(0x33), changed)
            .unwrap()
            .canonical_bytes();
        assert_ne!(bytes, initial, "material field {field} was not bound");
    }
    for (revision, generation) in [(REVISION + 1, GENERATION), (REVISION, GENERATION + 1)] {
        let bytes = BindingStatement::new(owner(revision, generation), user(0x33), material())
            .unwrap()
            .canonical_bytes();
        assert_ne!(bytes, initial, "account counters were not bound");
    }
    let another = OwnerAccount::new(user(0x66), Role::Owner, true, REVISION, GENERATION).unwrap();
    assert_ne!(
        BindingStatement::new(another, user(0x66), material())
            .unwrap()
            .canonical_bytes(),
        initial
    );
}

#[test]
fn withdrawal_is_terminal_and_checks_the_exact_binding_revision() {
    let record = BindingRecord::registered(statement());
    for expected in [0, 2, u32::MAX] {
        assert_eq!(
            record.withdraw(owner(REVISION, GENERATION), expected),
            Err(BindingError::RevisionConflict)
        );
    }
    let withdrawn = record.withdraw(owner(REVISION, GENERATION), 1).unwrap();
    for expected in [1, 2] {
        assert_eq!(
            withdrawn.withdraw(owner(REVISION, GENERATION), expected),
            Err(BindingError::Withdrawn)
        );
    }
    assert_eq!(withdrawn.registration(), record.registration());
}

#[test]
fn withdrawal_rejects_rolled_back_account_counters() {
    let record = BindingRecord::registered(statement());
    for (revision, generation) in [(REVISION - 1, GENERATION), (REVISION, GENERATION - 1)] {
        assert_eq!(
            record.withdraw(owner(revision, generation), 1),
            Err(BindingError::StaleAccount)
        );
    }
    assert_eq!(record.revision(), 1);
    assert!(record.withdrawal().is_none());
}
