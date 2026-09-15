#[allow(dead_code)]
mod document_store_support;

use application::documents::{
    validate_record_with_ports, CaseDocumentStore, DocumentAction, DocumentRecord,
    DocumentValidationPorts, VersionSelection,
};
use application::vault::{decrypt_with_ports, encrypt_with_ports, VaultFile};
use domain::crypto::{DocumentHasher, DocumentId, DocumentVersion, KeyManager, Sha256Digest};
use domain::identity::Role;
use infrastructure::{
    EnvelopeKeyManager, PostgresCaseDocumentStore, Rfc3161Verifier, RingAesGcmCipher,
    RingSha256Hasher, RsaPkcs1Verifier, X509ChainValidator,
};
use time::OffsetDateTime;
use zeroize::Zeroizing;

use document_store_support::{case, database_url, user};

#[test]
fn persisted_versions_use_fresh_keys_and_reject_vault_identity_version_or_digest_substitution() {
    let Some(url) = database_url() else { return };
    let owner = user(&url, Role::Owner);
    let case = case(&url, owner);
    let store = PostgresCaseDocumentStore::connect(&url).unwrap();
    let at = OffsetDateTime::now_utc();
    let id = DocumentId::new();
    let kek = [0x39_u8; 32];
    let cipher = RingAesGcmCipher::new();
    let keys = EnvelopeKeyManager::new();
    let hasher = RingSha256Hasher::new();
    let mut records = Vec::new();
    let mut data_keys = Vec::new();
    for (index, bytes) in [
        b"identical content".as_slice(),
        b"identical content",
        b"different content",
    ]
    .into_iter()
    .enumerate()
    {
        let version = DocumentVersion::new(index as u32 + 1).unwrap();
        let vault = encrypt_with_ports(&cipher, &keys, &kek, bytes, id, version).unwrap();
        let parsed = VaultFile::parse(&vault).unwrap();
        data_keys.push(Zeroizing::new(
            keys.unwrap_dek(&kek, &parsed.wrapped_dek).unwrap(),
        ));
        let record = DocumentRecord::pending(
            id,
            version,
            format!("v{}.txt", version.get()),
            hasher.hash_bytes(bytes),
            vault,
        )
        .unwrap();
        if index == 0 {
            store.insert(owner, case, record.clone(), at).unwrap();
        } else {
            store
                .append(
                    owner,
                    case,
                    DocumentVersion::new(index as u32).unwrap(),
                    record.clone(),
                    at,
                )
                .unwrap();
        }
        let loaded = store
            .load(
                owner,
                case,
                id,
                VersionSelection::Exact(version),
                DocumentAction::Verify,
            )
            .unwrap();
        assert_eq!(loaded, record);
        assert_eq!(
            decrypt_with_ports(&cipher, &keys, &kek, &loaded.vault, id, version).unwrap(),
            bytes
        );
        records.push(record);
    }
    for i in 0..records.len() {
        for j in i + 1..records.len() {
            assert_ne!(records[i].vault, records[j].vault);
            assert_ne!(*data_keys[i], *data_keys[j]);
        }
    }
    let ports = DocumentValidationPorts {
        cipher: &cipher,
        keys: &keys,
        hasher: &hasher,
        signature_verifier: &RsaPkcs1Verifier::new(),
        certificate_validator: &X509ChainValidator::new(),
        timestamp_verifier: &Rfc3161Verifier::new(),
    };
    for record in &records {
        validate_record_with_ports(record, &kek, &ports).unwrap();
    }
    let mut changes = vec![records[0].clone(); 4];
    changes[0].vault = records[1].vault.clone();
    changes[1].version = records[1].version;
    changes[2].id = DocumentId::new();
    changes[3].digest = Sha256Digest::from_array([0; 32]);
    for changed in changes {
        assert!(validate_record_with_ports(&changed, &kek, &ports).is_err());
    }
}
