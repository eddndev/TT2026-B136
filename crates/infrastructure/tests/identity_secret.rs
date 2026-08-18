use application::identity::SecretProtector;
use domain::identity::UserId;
use infrastructure::AesGcmSecretProtector;
use uuid::Uuid;
use zeroize::Zeroizing;

#[test]
fn protected_totp_secret_round_trips_only_for_the_bound_user() {
    let protector = AesGcmSecretProtector::new(Zeroizing::new(vec![0x42; 32])).unwrap();
    let owner = UserId::from_uuid(Uuid::from_u128(1));
    let other = UserId::from_uuid(Uuid::from_u128(2));
    let protected = protector.protect(owner, b"twenty-byte-secret!!").unwrap();

    assert_ne!(protected, b"twenty-byte-secret!!");
    assert_eq!(
        protector.expose(owner, &protected).unwrap().as_slice(),
        b"twenty-byte-secret!!"
    );
    assert!(protector.expose(other, &protected).is_err());
}

#[test]
fn protector_rejects_a_non_aes256_key() {
    assert!(AesGcmSecretProtector::new(Zeroizing::new(vec![0; 16])).is_err());
}
