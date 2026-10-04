use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
    owner_certificate_login::{LoginAccount, LoginNonce, LoginStatement},
    owner_certificates::BindingMaterial,
};
use uuid::Uuid;

pub const GENERATION: u64 = 0x0102_0304_0506_0708;
pub const ISSUED: i64 = 1_700_000_000;
pub const EXPIRES: i64 = 1_700_000_300;
pub const NONCE: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

pub const LITERAL: &str = concat!(
    "4f574e41555448310101",
    "11111111111111111111111111111111",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "01020304",
    "33333333333333333333333333333333",
    "0102030405060708",
    "44444444444444444444444444444444",
    "5555555555555555555555555555555555555555555555555555555555555555",
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    "000000006553f100",
    "000000006553f22c"
);

pub const MAXIMUM_LITERAL: &str = concat!(
    "4f574e41555448310101",
    "11111111111111111111111111111111",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "ffffffff",
    "33333333333333333333333333333333",
    "7fffffffffffffff",
    "44444444444444444444444444444444",
    "5555555555555555555555555555555555555555555555555555555555555555",
    "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "7ffffffffffffed3",
    "7fffffffffffffff"
);

pub fn user(byte: u8) -> UserId {
    UserId::from_uuid(Uuid::from_bytes([byte; 16]))
}

pub fn account() -> LoginAccount {
    LoginAccount::new(user(0x33), Role::Owner, true, GENERATION).unwrap()
}

pub fn material(trust_revision: u32) -> BindingMaterial {
    BindingMaterial::new(
        Uuid::from_bytes([0x11; 16]),
        Uuid::from_bytes([0x44; 16]),
        Sha256Digest::from_array([0x22; 32]),
        Sha256Digest::from_array([0x55; 32]),
        trust_revision,
    )
    .unwrap()
}

pub fn statement() -> LoginStatement {
    LoginStatement::new(
        account(),
        material(0x0102_0304),
        LoginNonce::from_bytes(&NONCE).unwrap(),
        ISSUED,
        EXPIRES,
    )
    .unwrap()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}
