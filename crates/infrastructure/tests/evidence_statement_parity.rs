//! Parity of the canonical binding and consent statements between the domain
//! codec in `crates/domain/src/document_signing.rs` and the independent
//! decoder of `evidence_verify::statements`, which the browser validator
//! uses. Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::document_signing::{
    DocumentConsent, SignerAccount, SigningBindingMaterial, SigningBindingStatement, Uuid,
};
use domain::identity::{Role, UserId};
use evidence_verify::statements::{
    decode_binding, decode_consent, encode_binding, encode_consent, BindingStatement,
    ConsentStatement,
};

/// Every field both statements can carry.
#[derive(Clone, Copy)]
struct Values {
    deployment: [u8; 16],
    account: [u8; 16],
    binding: [u8; 16],
    operation: [u8; 16],
    case: [u8; 16],
    document: [u8; 16],
    root: [u8; 32],
    leaf: [u8; 32],
    content: [u8; 32],
    signature: [u8; 32],
    account_revision: u64,
    generation: u64,
    trust_revision: u32,
    version: u32,
}

/// Distinct field values: ordinary, the largest admitted and the smallest.
fn samples() -> Vec<Values> {
    let ordinary = Values {
        deployment: pattern(0x10),
        account: pattern(0x20),
        binding: pattern(0x30),
        operation: pattern(0x40),
        case: pattern(0x50),
        document: pattern(0x60),
        root: wide(0x70),
        leaf: wide(0x80),
        content: wide(0x90),
        signature: wide(0xa0),
        account_revision: 7,
        generation: 2,
        trust_revision: 3,
        version: 4,
    };
    let largest = Values {
        deployment: [0xff; 16],
        account: [0xff; 16],
        binding: [0xff; 16],
        operation: [0xff; 16],
        case: [0xff; 16],
        document: [0xff; 16],
        root: [0xff; 32],
        leaf: [0xff; 32],
        content: [0xff; 32],
        signature: [0xff; 32],
        account_revision: i64::MAX as u64,
        generation: i64::MAX as u64,
        trust_revision: u32::MAX,
        version: u32::MAX,
    };
    let mut one = [0u8; 16];
    one[15] = 1;
    let smallest = Values {
        deployment: one,
        account: one,
        binding: one,
        operation: one,
        case: one,
        document: one,
        root: [0; 32],
        leaf: [0; 32],
        content: [0; 32],
        signature: [0; 32],
        account_revision: 0,
        generation: 0,
        trust_revision: 1,
        version: 1,
    };
    vec![ordinary, largest, smallest]
}

fn pattern(seed: u8) -> [u8; 16] {
    std::array::from_fn(|index| seed.wrapping_add(index as u8))
}

fn wide(seed: u8) -> [u8; 32] {
    std::array::from_fn(|index| seed.wrapping_mul(3).wrapping_add(index as u8))
}

fn account(values: &Values) -> SignerAccount {
    SignerAccount::new(
        UserId::from_uuid(Uuid::from_bytes(values.account)),
        Role::Litigator,
        true,
        values.account_revision,
        values.generation,
    )
    .expect("admitted account")
}

fn material(values: &Values) -> SigningBindingMaterial {
    SigningBindingMaterial::new(
        Uuid::from_bytes(values.deployment),
        Uuid::from_bytes(values.binding),
        Sha256Digest::from_array(values.root),
        Sha256Digest::from_array(values.leaf),
        values.trust_revision,
    )
    .expect("admitted material")
}

fn domain_binding(values: &Values) -> SigningBindingStatement {
    let account = account(values);
    SigningBindingStatement::new(account, account.id(), material(values)).expect("binding")
}

fn domain_consent(values: &Values) -> DocumentConsent {
    DocumentConsent::new(
        account(values),
        material(values),
        Uuid::from_bytes(values.operation),
        CaseId::from_uuid(Uuid::from_bytes(values.case)),
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_bytes(values.document)),
            version: DocumentVersion::new(values.version).expect("version"),
        },
        Sha256Digest::from_array(values.content),
        Sha256Digest::from_array(values.signature),
    )
    .expect("consent")
}

fn verifier_binding(values: &Values) -> BindingStatement {
    BindingStatement {
        deployment: values.deployment,
        root_sha256: values.root,
        trust_revision: values.trust_revision,
        account: values.account,
        account_revision: values.account_revision,
        authentication_generation: values.generation,
        binding: values.binding,
        certificate_sha256: values.leaf,
    }
}

fn verifier_consent(values: &Values) -> ConsentStatement {
    ConsentStatement {
        deployment: values.deployment,
        root_sha256: values.root,
        trust_revision: values.trust_revision,
        account: values.account,
        account_revision: values.account_revision,
        authentication_generation: values.generation,
        binding: values.binding,
        certificate_sha256: values.leaf,
        operation: values.operation,
        case: values.case,
        document: values.document,
        document_version: values.version,
        document_sha256: values.content,
        signature_sha256: values.signature,
    }
}

/// Field by field equality of the shared prefix.
fn assert_same_prefix(
    account: &SignerAccount,
    material: &SigningBindingMaterial,
    decoded: &BindingStatement,
) {
    assert_eq!(material.deployment().as_bytes(), &decoded.deployment);
    assert_eq!(material.root().as_bytes(), &decoded.root_sha256);
    assert_eq!(material.trust_revision(), decoded.trust_revision);
    assert_eq!(account.id().as_uuid().as_bytes(), &decoded.account);
    assert_eq!(account.revision(), decoded.account_revision);
    assert_eq!(account.generation(), decoded.authentication_generation);
    assert_eq!(material.binding().as_bytes(), &decoded.binding);
    assert_eq!(material.leaf().as_bytes(), &decoded.certificate_sha256);
}

fn assert_same_binding(domain: &SigningBindingStatement, decoded: &BindingStatement) {
    assert_same_prefix(domain.account(), domain.material(), decoded);
}

fn assert_same_consent(domain: &DocumentConsent, decoded: &ConsentStatement) {
    let prefix = BindingStatement {
        deployment: decoded.deployment,
        root_sha256: decoded.root_sha256,
        trust_revision: decoded.trust_revision,
        account: decoded.account,
        account_revision: decoded.account_revision,
        authentication_generation: decoded.authentication_generation,
        binding: decoded.binding,
        certificate_sha256: decoded.certificate_sha256,
    };
    assert_same_prefix(domain.account(), domain.material(), &prefix);
    assert_eq!(domain.operation().as_bytes(), &decoded.operation);
    assert_eq!(domain.case().as_uuid().as_bytes(), &decoded.case);
    assert_eq!(domain.document().id.as_uuid().as_bytes(), &decoded.document);
    assert_eq!(domain.document().version.get(), decoded.document_version);
    assert_eq!(domain.content_digest().as_bytes(), &decoded.document_sha256);
    assert_eq!(
        domain.pdf_signature_digest().as_bytes(),
        &decoded.signature_sha256
    );
}

#[test]
fn domain_binding_bytes_decode_to_the_same_fields() {
    for values in samples() {
        let domain = domain_binding(&values);
        let bytes = domain.canonical_bytes();
        assert_eq!(bytes, encode_binding(&verifier_binding(&values)));
        let decoded = decode_binding(&bytes).expect("verifier decodes domain bytes");
        assert_same_binding(&domain, &decoded);
        assert_eq!(decoded, verifier_binding(&values));
    }
}

#[test]
fn domain_consent_bytes_decode_to_the_same_fields() {
    for values in samples() {
        let domain = domain_consent(&values);
        let bytes = domain.canonical_bytes();
        assert_eq!(bytes, encode_consent(&verifier_consent(&values)));
        let decoded = decode_consent(&bytes).expect("verifier decodes domain bytes");
        assert_same_consent(&domain, &decoded);
        assert_eq!(decoded, verifier_consent(&values));
    }
}

#[test]
fn verifier_encodings_decode_in_the_domain_to_the_same_fields() {
    for values in samples() {
        let binding = verifier_binding(&values);
        let domain = SigningBindingStatement::from_canonical_bytes(&encode_binding(&binding))
            .expect("domain decodes verifier binding");
        assert_same_binding(&domain, &binding);
        let consent = verifier_consent(&values);
        let domain = DocumentConsent::from_canonical_bytes(&encode_consent(&consent))
            .expect("domain decodes verifier consent");
        assert_same_consent(&domain, &consent);
    }
}

/// One named edit of statement bytes.
type Mutation = (&'static str, Box<dyn Fn(&mut Vec<u8>)>);

/// Named edits that make a valid statement unacceptable to the domain.
fn mutations(consent: bool) -> Vec<Mutation> {
    let mut list: Vec<Mutation> = vec![
        (
            "one byte short",
            Box::new(|bytes| bytes.truncate(bytes.len() - 1)),
        ),
        ("one byte long", Box::new(|bytes| bytes.push(0))),
        ("empty", Box::new(|bytes| bytes.clear())),
        ("magic", Box::new(|bytes| bytes[0] ^= 0x20)),
        (
            "other purpose magic",
            Box::new(move |bytes| {
                let magic: &[u8; 8] = if consent { b"DOCBIND1" } else { b"DOCSIGN1" };
                bytes[..8].copy_from_slice(magic);
            }),
        ),
        ("purpose number", Box::new(|bytes| bytes[8] = 2)),
        ("format version", Box::new(|bytes| bytes[9] = 0)),
        ("nil deployment", Box::new(|bytes| zero(bytes, 10, 16))),
        ("nil account", Box::new(|bytes| zero(bytes, 62, 16))),
        ("nil binding", Box::new(|bytes| zero(bytes, 94, 16))),
        ("zero trust revision", Box::new(|bytes| zero(bytes, 58, 4))),
        (
            "revision above i64",
            Box::new(|bytes| bytes[78..86].copy_from_slice(&(1u64 << 63).to_be_bytes())),
        ),
        (
            "generation above revision",
            Box::new(|bytes| {
                bytes[78..86].copy_from_slice(&5u64.to_be_bytes());
                bytes[86..94].copy_from_slice(&6u64.to_be_bytes());
            }),
        ),
    ];
    if consent {
        list.push(("nil operation", Box::new(|bytes| zero(bytes, 142, 16))));
        list.push(("nil case", Box::new(|bytes| zero(bytes, 158, 16))));
        list.push(("nil document", Box::new(|bytes| zero(bytes, 174, 16))));
        list.push((
            "zero document version",
            Box::new(|bytes| zero(bytes, 190, 4)),
        ));
    }
    list
}

fn zero(bytes: &mut [u8], offset: usize, length: usize) {
    bytes[offset..offset + length].fill(0);
}

#[test]
fn mutated_bindings_are_rejected_by_both_decoders() {
    for values in samples() {
        let valid = domain_binding(&values).canonical_bytes().to_vec();
        for (name, mutate) in mutations(false) {
            let mut bytes = valid.clone();
            mutate(&mut bytes);
            assert!(
                SigningBindingStatement::from_canonical_bytes(&bytes).is_err(),
                "domain admits binding with {name}"
            );
            assert!(
                decode_binding(&bytes).is_err(),
                "verifier admits binding with {name}"
            );
        }
    }
}

#[test]
fn mutated_consents_are_rejected_by_both_decoders() {
    for values in samples() {
        let valid = domain_consent(&values).canonical_bytes().to_vec();
        for (name, mutate) in mutations(true) {
            let mut bytes = valid.clone();
            mutate(&mut bytes);
            assert!(
                DocumentConsent::from_canonical_bytes(&bytes).is_err(),
                "domain admits consent with {name}"
            );
            assert!(
                decode_consent(&bytes).is_err(),
                "verifier admits consent with {name}"
            );
        }
    }
}

/// Every single-byte replacement of a valid statement is admitted or
/// rejected by both decoders alike, and admitted bytes carry equal fields.
#[test]
fn every_single_byte_change_gets_the_same_verdict() {
    for values in samples() {
        let binding = domain_binding(&values).canonical_bytes();
        let consent = domain_consent(&values).canonical_bytes();
        for replacement in [0x00u8, 0x01, 0x80, 0xff] {
            for index in 0..binding.len() {
                let mut bytes = binding;
                bytes[index] = replacement;
                let domain = SigningBindingStatement::from_canonical_bytes(&bytes);
                let decoded = decode_binding(&bytes);
                assert_eq!(domain.is_ok(), decoded.is_ok(), "binding byte {index}");
                if let (Ok(domain), Ok(decoded)) = (domain, decoded) {
                    assert_same_binding(&domain, &decoded);
                }
            }
            for index in 0..consent.len() {
                let mut bytes = consent;
                bytes[index] = replacement;
                let domain = DocumentConsent::from_canonical_bytes(&bytes);
                let decoded = decode_consent(&bytes);
                assert_eq!(domain.is_ok(), decoded.is_ok(), "consent byte {index}");
                if let (Ok(domain), Ok(decoded)) = (domain, decoded) {
                    assert_same_consent(&domain, &decoded);
                }
            }
        }
    }
}
