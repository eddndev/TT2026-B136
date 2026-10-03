use std::{fs, sync::OnceLock};

use application::credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot};
use der::{asn1::OctetString, oid::AssociatedOid, Decode, Encode};
use domain::{
    crypto::{
        CredentialCheck, DocumentHasher, DocumentSigner, InternalDeclarationVerifier, Sha256Digest,
        Signature,
    },
    identity::{Role, UserId},
    owner_certificates::{BindingMaterial, BindingStatement, OwnerAccount},
};
use infrastructure::{
    certificates::{
        InternalRsaDeclarationVerifier, InternalRsaOwnerBindingVerifier, OwnerBindingFailure,
    },
    RingSha256Hasher, RsaPkcs1Signer,
};
use time::OffsetDateTime;
use uuid::Uuid;
use x509_cert::Certificate;
use zeroize::Zeroizing;

use crate::declaration_fixture::{fixture as declaration, openssl, script};

pub struct Fixture {
    pub leaf: Vec<u8>,
    pub leaf_der: Vec<u8>,
    pub statement: BindingStatement,
    pub signature: Signature,
    pub trust: CredentialTrustSnapshot,
    pub at: i64,
}

pub fn owner(revision: u64, generation: u64) -> OwnerAccount {
    OwnerAccount::new(
        UserId::from_uuid(Uuid::from_bytes([0x33; 16])),
        Role::Owner,
        true,
        revision,
        generation,
    )
    .unwrap()
}

pub fn statement(
    trust: &CredentialTrustSnapshot,
    leaf: Sha256Digest,
    account: OwnerAccount,
    binding: Uuid,
) -> BindingStatement {
    BindingStatement::new(
        account,
        UserId::from_uuid(Uuid::from_bytes([0x33; 16])),
        BindingMaterial::new(
            trust.deployment_id,
            binding,
            trust.inspection.root_fingerprint,
            leaf,
            trust.revision.get(),
        )
        .unwrap(),
    )
    .unwrap()
}

pub fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let base = declaration();
        script("issue-cert.sh", &base.ca, &["Synthetic Owner Partner"]);
        let leaf = fs::read(base.ca.join("certs/synthetic-owner-partner.crt.pem")).unwrap();
        let (_, leaf_der) = der::pem::decode_vec(&leaf).unwrap();
        let trust = CredentialTrustSnapshot {
            deployment_id: Uuid::from_bytes([0x11; 16]),
            revision: CredentialTrustRevision::new(7).unwrap(),
            inspection: InternalRsaDeclarationVerifier
                .inspect_trust(&base.root, &base.crl, base.at)
                .unwrap(),
            published_at: OffsetDateTime::from_unix_timestamp(base.at - 60).unwrap(),
            published_by: "synthetic-admin".into(),
        };
        // This supplied snapshot is not a PostgreSQL publication proof.
        let statement = statement(
            &trust,
            RingSha256Hasher.hash_bytes(&leaf_der),
            owner(9, 4),
            Uuid::from_bytes([0x44; 16]),
        );
        let statement_path = base.ca.join("owner-binding.bin");
        fs::write(&statement_path, statement.canonical_bytes()).unwrap();
        let signature = Signature::from_bytes(openssl(&[
            "dgst",
            "-sha256",
            "-sign",
            base.ca
                .join("private/synthetic-owner-partner.key.pem")
                .to_str()
                .unwrap(),
            statement_path.to_str().unwrap(),
        ]))
        .unwrap();
        Fixture {
            leaf,
            leaf_der,
            statement,
            signature,
            trust,
            at: base.at,
        }
    })
}

pub fn partner() -> Certificate {
    Certificate::from_der(&fixture().leaf_der).unwrap()
}

pub fn sign(bytes: &[u8]) -> Signature {
    let key = Zeroizing::new(
        fs::read(
            declaration()
                .ca
                .join("private/synthetic-owner-partner.key.pem"),
        )
        .unwrap(),
    );
    RsaPkcs1Signer::new(key)
        .unwrap()
        .sign(&RingSha256Hasher.hash_bytes(bytes))
        .unwrap()
}

pub fn change_extension<T: AssociatedOid + Encode>(certificate: &mut Certificate, value: T) {
    certificate
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == T::OID)
        .unwrap()
        .extn_value = OctetString::new(value.to_der().unwrap()).unwrap();
}

pub fn verify(
    statement: &BindingStatement,
    certificate: &[u8],
    signature: &Signature,
    trust: &CredentialTrustSnapshot,
    at: i64,
) -> Result<CredentialCheck, OwnerBindingFailure> {
    InternalRsaOwnerBindingVerifier::new().verify_registration(
        statement,
        certificate,
        signature,
        trust,
        at,
    )
}

pub fn trust_with(root: &[u8], crl: &[u8]) -> CredentialTrustSnapshot {
    let mut trust = fixture().trust.clone();
    trust.inspection = InternalRsaDeclarationVerifier
        .inspect_trust(root, crl, fixture().at)
        .unwrap();
    trust
}

pub fn verify_leaf(certificate: &[u8], at: i64) -> Result<CredentialCheck, OwnerBindingFailure> {
    let fx = fixture();
    let statement = statement(
        &fx.trust,
        RingSha256Hasher.hash_bytes(certificate),
        owner(9, 4),
        Uuid::from_bytes([0x44; 16]),
    );
    let signature = sign(&statement.canonical_bytes());
    verify(&statement, certificate, &signature, &fx.trust, at)
}
