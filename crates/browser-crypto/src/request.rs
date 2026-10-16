use crate::{certificate, keys, CryptoError, Result};
use der::{
    asn1::{Any, BitString, ObjectIdentifier},
    Decode, DecodePem, Encode, EncodePem, Tag,
};
use pkcs8::{
    spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned},
    EncodePublicKey,
};
use ring::signature::{UnparsedPublicKey, RSA_PKCS1_2048_8192_SHA256};
use x509_cert::{
    attr::AttributeTypeAndValue,
    name::{Name, RdnSequence, RelativeDistinguishedName},
    request::{CertReq, CertReqInfo, Version},
};

const SHA256_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");

pub(crate) fn subject(cn: &str, organization: &str, country: &str) -> Result<Name> {
    if [cn, organization].iter().any(|value| {
        value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control)
    }) || country.len() != 2
        || !country.bytes().all(|value| value.is_ascii_uppercase())
    {
        return Err(CryptoError("invalid_subject"));
    }
    let values = [
        ("2.5.4.6", country, Tag::PrintableString),
        ("2.5.4.10", organization, Tag::Utf8String),
        ("2.5.4.3", cn, Tag::Utf8String),
    ];
    let mut rdns = Vec::with_capacity(values.len());
    for (oid, value, tag) in values {
        let attribute = AttributeTypeAndValue {
            oid: ObjectIdentifier::new(oid).map_err(|_| CryptoError("invalid_subject"))?,
            value: Any::new(tag, value.as_bytes()).map_err(|_| CryptoError("invalid_subject"))?,
        };
        rdns.push(
            RelativeDistinguishedName::try_from(vec![attribute])
                .map_err(|_| CryptoError("invalid_subject"))?,
        );
    }
    Ok(RdnSequence(rdns))
}

pub(crate) fn create(
    key: &rsa::RsaPrivateKey,
    signer: &ring::rsa::KeyPair,
    subject: Name,
) -> Result<String> {
    let spki = key
        .to_public_key()
        .to_public_key_der()
        .map_err(|_| CryptoError("generation_failed"))?;
    let info = CertReqInfo {
        version: Version::V1,
        subject,
        public_key: SubjectPublicKeyInfoOwned::from_der(spki.as_bytes())
            .map_err(|_| CryptoError("generation_failed"))?,
        attributes: Default::default(),
    };
    let signature = keys::sign(
        signer,
        &info
            .to_der()
            .map_err(|_| CryptoError("generation_failed"))?,
    )?;
    let request = CertReq {
        info,
        algorithm: AlgorithmIdentifierOwned {
            oid: SHA256_RSA,
            parameters: Some(
                Any::new(Tag::Null, Vec::<u8>::new())
                    .map_err(|_| CryptoError("generation_failed"))?,
            ),
        },
        signature: BitString::from_bytes(&signature)
            .map_err(|_| CryptoError("generation_failed"))?,
    };
    request
        .to_pem(der::pem::LineEnding::LF)
        .map_err(|_| CryptoError("generation_failed"))
}

pub(crate) fn parse(pem: &str) -> Result<CertReq> {
    if pem.len() > 16_384 {
        return Err(CryptoError("input_too_large"));
    }
    let invalid = CryptoError("invalid_csr");
    let request = CertReq::from_pem(pem).map_err(|_| invalid)?;
    if request.algorithm.oid != SHA256_RSA
        || request
            .algorithm
            .parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_null())
    {
        return Err(invalid);
    }
    certificate::public_key(&request.info.public_key)?;
    let public = request
        .info
        .public_key
        .subject_public_key
        .as_bytes()
        .ok_or(invalid)?;
    let signature = request.signature.as_bytes().ok_or(invalid)?;
    UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA256, public)
        .verify(&request.info.to_der().map_err(|_| invalid)?, signature)
        .map_err(|_| invalid)?;
    Ok(request)
}
