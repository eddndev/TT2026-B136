use std::env;

use application::cases::{CaseRecord, CaseRepository};
use application::documents::{DocumentRecord, SealedEvidence};
use domain::cases::CaseId;
use domain::crypto::{
    DocumentId, DocumentVersion, RecoveryCodeSet, Sha256Digest, RECOVERY_CODE_COUNT,
};
use domain::identity::{Role, UserId};
use infrastructure::{PostgresCaseRepository, PostgresUserRepository};
use postgres::{Client, NoTls};

pub fn database_url() -> Option<String> {
    let url = env::var("DOCUMENT_TEST_DATABASE_URL").ok();
    if url.is_none() {
        eprintln!("skipping document database test: DOCUMENT_TEST_DATABASE_URL is unset");
    }
    url
}

pub fn user(url: &str, role: Role) -> UserId {
    let id = UserId::new();
    PostgresUserRepository::connect(url).unwrap();
    let codes = RecoveryCodeSet::from_hashes(
        (0..RECOVERY_CODE_COUNT)
            .map(|index| format!("$recovery-{index}"))
            .collect(),
    )
    .unwrap();
    Client::connect(url, NoTls).unwrap().execute(
        "INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes) VALUES($1,$2,$3,$4,$5,$6)",
        &[&id.as_uuid(), &format!("{id}@example.com"), &"$argon2id$test", &role.as_str(), &vec![7u8;48], &serde_json::to_value(codes).unwrap()],
    ).unwrap();
    id
}

pub fn case(url: &str, creator: UserId) -> CaseId {
    let record = CaseRecord {
        id: CaseId::new(),
        title: "Document authorization".into(),
        reference: "File 42".into(),
        created_by: creator,
    };
    PostgresCaseRepository::connect(url)
        .unwrap()
        .insert(record.clone())
        .unwrap();
    record.id
}

pub fn document() -> DocumentRecord {
    DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "evidence.txt".into(),
        Sha256Digest::from_array([3; 32]),
        vec![8; 80],
    )
    .unwrap()
}

pub fn evidence() -> SealedEvidence {
    SealedEvidence {
        signature: vec![1, 2],
        timestamp_token: vec![3, 4],
        signer_certificate_pem: vec![5],
        issuer_certificate_pem: vec![6],
        crl_pem: vec![7],
        tsa_chain_pem: Some(vec![8]),
        openssl_version: "OpenSSL test".into(),
    }
}
