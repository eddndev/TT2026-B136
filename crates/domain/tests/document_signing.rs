use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    document_signing::{
        DocumentConsent, SignerAccount, SigningBindingMaterial, SigningBindingStatement,
        SigningError,
    },
    identity::{Role, UserId},
};
use uuid::Uuid;

const REVISION: u64 = 0x1112_1314_1516_1718;
const GENERATION: u64 = 0x0102_0304_0506_0708;

fn uuid(byte: u8) -> Uuid {
    Uuid::from_bytes([byte; 16])
}

fn user(byte: u8) -> UserId {
    UserId::from_uuid(uuid(byte))
}

fn account() -> SignerAccount {
    SignerAccount::new(user(0x33), Role::Litigator, true, REVISION, GENERATION).unwrap()
}

fn material() -> SigningBindingMaterial {
    SigningBindingMaterial::new(
        uuid(0x11),
        uuid(0x44),
        Sha256Digest::from_array([0x22; 32]),
        Sha256Digest::from_array([0x55; 32]),
        0x0102_0304,
    )
    .unwrap()
}

fn document() -> DocumentVersionRef {
    DocumentVersionRef {
        id: DocumentId::from_uuid(uuid(0x88)),
        version: DocumentVersion::new(0x0506_0708).unwrap(),
    }
}

fn consent() -> DocumentConsent {
    DocumentConsent::new(
        account(),
        material(),
        uuid(0x66),
        CaseId::from_uuid(uuid(0x77)),
        document(),
        Sha256Digest::from_array([0x99; 32]),
        Sha256Digest::from_array([0xaa; 32]),
    )
    .unwrap()
}

fn binding() -> SigningBindingStatement {
    SigningBindingStatement::new(account(), user(0x33), material()).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

#[test]
fn certificate_binding_has_a_complete_literal_vector_and_distinct_owner_purpose() {
    let statement = binding();
    let bytes = statement.canonical_bytes();
    assert_eq!(bytes.len(), 142);
    assert_eq!(
        hex(&bytes),
        concat!(
            "444f4342494e44310101",
            "11111111111111111111111111111111",
            "2222222222222222222222222222222222222222222222222222222222222222",
            "01020304",
            "33333333333333333333333333333333",
            "1112131415161718",
            "0102030405060708",
            "44444444444444444444444444444444",
            "5555555555555555555555555555555555555555555555555555555555555555"
        )
    );
    assert_ne!(&bytes[..8], b"OWNCERT1");
    assert_eq!(statement.account(), &account());
    assert_eq!(statement.material(), &material());
    assert_eq!(statement.require_exact_bytes(&bytes), Ok(()));
}

#[test]
fn consent_literal_vector_binds_the_pdf_signature_and_exact_document_context() {
    let value = consent();
    let bytes = value.canonical_bytes();
    assert_eq!(bytes.len(), 258);
    assert_eq!(
        hex(&bytes),
        concat!(
            "444f435349474e310101",
            "11111111111111111111111111111111",
            "2222222222222222222222222222222222222222222222222222222222222222",
            "01020304",
            "33333333333333333333333333333333",
            "1112131415161718",
            "0102030405060708",
            "44444444444444444444444444444444",
            "5555555555555555555555555555555555555555555555555555555555555555",
            "66666666666666666666666666666666",
            "77777777777777777777777777777777",
            "88888888888888888888888888888888",
            "05060708",
            "9999999999999999999999999999999999999999999999999999999999999999",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        )
    );
    assert_eq!(value.account(), &account());
    assert_eq!(value.material(), &material());
    assert_eq!(value.operation(), uuid(0x66));
    assert_eq!(value.case(), CaseId::from_uuid(uuid(0x77)));
    assert_eq!(value.document(), document());
    assert_eq!(value.content_digest(), Sha256Digest::from_array([0x99; 32]));
    assert_eq!(
        value.pdf_signature_digest(),
        Sha256Digest::from_array([0xaa; 32])
    );
    assert_eq!(value.require_exact_bytes(&bytes), Ok(()));
}

#[test]
fn only_an_active_litigator_can_describe_the_same_signing_account() {
    for role in [Role::Owner, Role::Paralegal, Role::Client] {
        assert_eq!(
            SignerAccount::new(user(0x33), role, true, 0, 0),
            Err(SigningError::LitigatorRequired)
        );
    }
    assert_eq!(
        SignerAccount::new(user(0x33), Role::Litigator, false, 0, 0),
        Err(SigningError::LitigatorRequired)
    );
    assert_eq!(
        SigningBindingStatement::new(account(), user(0x34), material()),
        Err(SigningError::DifferentAccount)
    );
    assert_eq!(
        SignerAccount::new(user(0), Role::Litigator, true, 0, 0),
        Err(SigningError::InvalidIdentity)
    );
}

#[test]
fn account_counters_preserve_zero_and_reject_unrepresentable_or_future_generations() {
    let maximum = i64::MAX as u64;
    for (revision, generation) in [(0, 0), (maximum, 0), (maximum, maximum)] {
        let value =
            SignerAccount::new(user(0x33), Role::Litigator, true, revision, generation).unwrap();
        assert_eq!(value.id(), user(0x33));
        assert_eq!(value.revision(), revision);
        assert_eq!(value.generation(), generation);
    }
    for (revision, generation) in [(0, 1), (maximum + 1, 0), (maximum, maximum + 1)] {
        assert_eq!(
            SignerAccount::new(user(0x33), Role::Litigator, true, revision, generation),
            Err(SigningError::InvalidAccountVersion)
        );
    }
}

#[test]
fn binding_requires_non_nil_public_references_and_positive_trust_revision() {
    for (deployment, binding) in [(Uuid::nil(), uuid(2)), (uuid(1), Uuid::nil())] {
        assert_eq!(
            SigningBindingMaterial::new(
                deployment,
                binding,
                Sha256Digest::from_array([0; 32]),
                Sha256Digest::from_array([0; 32]),
                1,
            ),
            Err(SigningError::InvalidIdentity)
        );
    }
    assert_eq!(
        SigningBindingMaterial::new(
            uuid(1),
            uuid(2),
            Sha256Digest::from_array([0; 32]),
            Sha256Digest::from_array([0; 32]),
            0,
        ),
        Err(SigningError::InvalidTrustRevision)
    );
    let value = material();
    assert_eq!(value.deployment(), uuid(0x11));
    assert_eq!(value.binding(), uuid(0x44));
    assert_eq!(value.root(), Sha256Digest::from_array([0x22; 32]));
    assert_eq!(value.leaf(), Sha256Digest::from_array([0x55; 32]));
    assert_eq!(value.trust_revision(), 0x0102_0304);
}

#[test]
fn consent_rejects_nil_operation_case_and_document_identities() {
    for field in 0..3 {
        let reference = DocumentVersionRef {
            id: DocumentId::from_uuid(if field == 2 { Uuid::nil() } else { uuid(0x88) }),
            version: DocumentVersion::initial(),
        };
        assert_eq!(
            DocumentConsent::new(
                account(),
                material(),
                if field == 0 { Uuid::nil() } else { uuid(0x66) },
                CaseId::from_uuid(if field == 1 { Uuid::nil() } else { uuid(0x77) }),
                reference,
                Sha256Digest::from_array([0x99; 32]),
                Sha256Digest::from_array([0xaa; 32]),
            ),
            Err(SigningError::InvalidIdentity)
        );
    }
}

#[test]
fn every_changed_consent_field_rejects_the_prior_signed_intention() {
    let original = consent().canonical_bytes();
    for field in 0..14 {
        let changed = DocumentConsent::new(
            SignerAccount::new(
                user(if field == 0 { 0x34 } else { 0x33 }),
                Role::Litigator,
                true,
                REVISION + u64::from(field == 1),
                GENERATION + u64::from(field == 2),
            )
            .unwrap(),
            SigningBindingMaterial::new(
                uuid(if field == 3 { 0x12 } else { 0x11 }),
                uuid(if field == 4 { 0x45 } else { 0x44 }),
                Sha256Digest::from_array([if field == 5 { 0x23 } else { 0x22 }; 32]),
                Sha256Digest::from_array([if field == 6 { 0x56 } else { 0x55 }; 32]),
                0x0102_0304 + u32::from(field == 7),
            )
            .unwrap(),
            uuid(if field == 8 { 0x67 } else { 0x66 }),
            CaseId::from_uuid(uuid(if field == 9 { 0x78 } else { 0x77 })),
            DocumentVersionRef {
                id: DocumentId::from_uuid(uuid(if field == 10 { 0x89 } else { 0x88 })),
                version: DocumentVersion::new(0x0506_0708 + u32::from(field == 11)).unwrap(),
            },
            Sha256Digest::from_array([if field == 12 { 0x9a } else { 0x99 }; 32]),
            Sha256Digest::from_array([if field == 13 { 0xab } else { 0xaa }; 32]),
        )
        .unwrap();
        assert_ne!(changed.canonical_bytes(), original, "unbound field {field}");
        assert_eq!(
            changed.require_exact_bytes(&original),
            Err(SigningError::StatementMismatch),
            "previous consent accepted for changed field {field}"
        );
    }
}

#[test]
fn changed_binding_identity_counters_and_trust_reject_the_prior_possession_proof() {
    let original = binding().canonical_bytes();
    for field in 0..8 {
        let id = user(if field == 0 { 0x34 } else { 0x33 });
        let changed = SigningBindingStatement::new(
            SignerAccount::new(
                id,
                Role::Litigator,
                true,
                REVISION + u64::from(field == 1),
                GENERATION + u64::from(field == 2),
            )
            .unwrap(),
            id,
            SigningBindingMaterial::new(
                uuid(if field == 3 { 0x12 } else { 0x11 }),
                uuid(if field == 4 { 0x45 } else { 0x44 }),
                Sha256Digest::from_array([if field == 5 { 0x23 } else { 0x22 }; 32]),
                Sha256Digest::from_array([if field == 6 { 0x56 } else { 0x55 }; 32]),
                0x0102_0304 + u32::from(field == 7),
            )
            .unwrap(),
        )
        .unwrap();
        assert_ne!(changed.canonical_bytes(), original, "unbound field {field}");
        assert_eq!(
            changed.require_exact_bytes(&original),
            Err(SigningError::StatementMismatch)
        );
    }
}

#[test]
fn one_changed_byte_or_another_purpose_never_matches_the_reconstructed_intention() {
    let statement = binding();
    let value = consent();
    for index in 0..142 {
        let mut bytes = statement.canonical_bytes();
        bytes[index] ^= 1;
        assert_eq!(
            statement.require_exact_bytes(&bytes),
            Err(SigningError::StatementMismatch)
        );
    }
    for index in 0..258 {
        let mut bytes = value.canonical_bytes();
        bytes[index] ^= 1;
        assert_eq!(
            value.require_exact_bytes(&bytes),
            Err(SigningError::StatementMismatch)
        );
    }
    assert_eq!(
        value.require_exact_bytes(&statement.canonical_bytes()),
        Err(SigningError::StatementMismatch)
    );
    assert_eq!(
        statement.require_exact_bytes(&value.canonical_bytes()),
        Err(SigningError::StatementMismatch)
    );
    for bytes in [
        Vec::new(),
        vec![0; 141],
        vec![0; 143],
        vec![0; 257],
        vec![0; 259],
    ] {
        assert_eq!(
            statement.require_exact_bytes(&bytes),
            Err(SigningError::StatementMismatch)
        );
        assert_eq!(
            value.require_exact_bytes(&bytes),
            Err(SigningError::StatementMismatch)
        );
    }
}

#[path = "document_signing_decode.rs"]
mod canonical_decode;
