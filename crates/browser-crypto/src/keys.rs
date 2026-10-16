use crate::{request, CryptoError, GeneratedKey, Result};
use der::{pem::LineEnding, Decode};
use pkcs5::pbes2::{EncryptionScheme, Kdf, Parameters, Pbkdf2Prf};
use pkcs8::{EncodePrivateKey, EncryptedPrivateKeyInfo, PrivateKeyInfo, SecretDocument};
use rand_chacha::ChaCha20Rng;
use ring::{
    rand::{SecureRandom, SystemRandom},
    rsa::KeyPair,
    signature,
};
use rsa::{pkcs1::DecodeRsaPublicKey, rand_core::SeedableRng, traits::PublicKeyParts};
use zeroize::Zeroizing;

const MAX_KEY_PEM: usize = 65_536;
const MAX_PASSWORD: usize = 1024;
const EXPORT_ITERATIONS: u32 = 600_000;
const MAX_IMPORT_ITERATIONS: u32 = 2_000_000;

fn password_valid(password: &str) -> Result<()> {
    if password.is_empty() || password.len() > MAX_PASSWORD {
        return Err(CryptoError("invalid_password"));
    }
    Ok(())
}

pub(crate) fn generate(
    password: &str,
    cn: &str,
    organization: &str,
    country: &str,
) -> Result<GeneratedKey> {
    generate_with_entropy(password, cn, organization, country, |bytes| {
        SystemRandom::new()
            .fill(bytes)
            .map_err(|_| CryptoError("entropy_unavailable"))
    })
}

fn generate_with_entropy(
    password: &str,
    cn: &str,
    organization: &str,
    country: &str,
    mut entropy: impl FnMut(&mut [u8]) -> Result<()>,
) -> Result<GeneratedKey> {
    password_valid(password)?;
    if password.chars().count() < 12 {
        return Err(CryptoError("invalid_password"));
    }
    let subject = request::subject(cn, organization, country)?;
    let mut seed = Zeroizing::new([0u8; 32]);
    entropy(seed.as_mut())?;
    let mut rng = ChaCha20Rng::from_seed(*seed);
    drop(seed);
    let key =
        rsa::RsaPrivateKey::new(&mut rng, 3072).map_err(|_| CryptoError("generation_failed"))?;
    let private = key
        .to_pkcs8_der()
        .map_err(|_| CryptoError("generation_failed"))?;
    let signing_key =
        KeyPair::from_pkcs8(private.as_bytes()).map_err(|_| CryptoError("generation_failed"))?;
    let csr_pem = request::create(&key, &signing_key, subject)?;
    let mut salt = [0u8; 16];
    let mut iv = [0u8; 16];
    entropy(&mut salt)?;
    entropy(&mut iv)?;
    let parameters = Parameters::pbkdf2_sha256_aes256cbc(EXPORT_ITERATIONS, &salt, &iv)
        .map_err(|_| CryptoError("generation_failed"))?;
    let encrypted = PrivateKeyInfo::from_der(private.as_bytes())
        .map_err(|_| CryptoError("generation_failed"))?
        .encrypt_with_params(parameters, password)
        .map_err(|_| CryptoError("generation_failed"))?;
    let encrypted_key_pem = encrypted
        .to_pem("ENCRYPTED PRIVATE KEY", LineEnding::LF)
        .map_err(|_| CryptoError("generation_failed"))?
        .to_string();
    Ok(GeneratedKey {
        encrypted_key_pem,
        csr_pem,
    })
}

pub(crate) fn unlock(encrypted_pem: &str, password: &str) -> Result<KeyPair> {
    password_valid(password)?;
    if encrypted_pem.len() > MAX_KEY_PEM {
        return Err(CryptoError("input_too_large"));
    }
    let (label, encoded) = SecretDocument::from_pem(encrypted_pem)
        .map_err(|_| CryptoError("invalid_key_or_password"))?;
    if label != "ENCRYPTED PRIVATE KEY" {
        return Err(CryptoError("unsupported_key_encryption"));
    }
    let encrypted = EncryptedPrivateKeyInfo::from_der(encoded.as_bytes())
        .map_err(|_| CryptoError("invalid_key_or_password"))?;
    validate_encryption(&encrypted)?;
    let private = encrypted
        .decrypt(password)
        .map_err(|_| CryptoError("invalid_key_or_password"))?;
    let key = KeyPair::from_pkcs8(private.as_bytes())
        .map_err(|_| CryptoError("invalid_key_or_password"))?;
    let public = rsa::RsaPublicKey::from_pkcs1_der(key.public().as_ref())
        .map_err(|_| CryptoError("unsupported_key"))?;
    if public.n().bits() != 3072 {
        return Err(CryptoError("unsupported_key"));
    }
    Ok(key)
}

fn validate_encryption(encrypted: &EncryptedPrivateKeyInfo<'_>) -> Result<()> {
    let invalid = CryptoError("unsupported_key_encryption");
    let parameters = encrypted.encryption_algorithm.pbes2().ok_or(invalid)?;
    let Kdf::Pbkdf2(kdf) = &parameters.kdf else {
        return Err(invalid);
    };
    if !(1..=MAX_IMPORT_ITERATIONS).contains(&kdf.iteration_count)
        || kdf.prf != Pbkdf2Prf::HmacWithSha256
        || kdf.salt.is_empty()
        || kdf.salt.len() > 64
        || kdf.key_length.is_some_and(|length| length != 32)
        || !matches!(parameters.encryption, EncryptionScheme::Aes256Cbc { .. })
    {
        return Err(invalid);
    }
    Ok(())
}

pub(crate) fn sign(key: &KeyPair, bytes: &[u8]) -> Result<Vec<u8>> {
    let mut signature = vec![0; key.public().modulus_len()];
    key.sign(
        &signature::RSA_PKCS1_SHA256,
        &SystemRandom::new(),
        bytes,
        &mut signature,
    )
    .map_err(|_| CryptoError("signing_failed"))?;
    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_entropy_returns_an_error_before_generating_a_key() {
        let result =
            generate_with_entropy("synthetic-test-password", "Holder", "Office", "MX", |_| {
                Err(CryptoError("entropy_unavailable"))
            });
        assert_eq!(result.err(), Some(CryptoError("entropy_unavailable")));
    }
}
