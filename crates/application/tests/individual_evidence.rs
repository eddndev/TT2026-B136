#[path = "support/individual_evidence.rs"]
mod support;

use application::documents::IndividualSigningEvidence;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use support::{fixture, resign, Hasher, Verifier};
use uuid::Uuid;

#[test]
fn historical_binding_and_consent_are_checked_at_their_captured_instants() {
    let (record, sealed, evidence) = fixture();
    let verifier = Verifier::default();
    evidence
        .validate(&record, &sealed, &Hasher, &verifier)
        .unwrap();
    assert_eq!(*verifier.calls.lock().unwrap(), [100, 300]);
    let bytes = serde_json::to_vec(&evidence).unwrap();
    let recovered: IndividualSigningEvidence = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(recovered, evidence);
    recovered
        .validate(&record, &sealed, &Hasher, &Verifier::default())
        .unwrap();
}

#[test]
fn document_version_content_and_detached_pdf_signature_must_match_consent() {
    let (record, sealed, evidence) = fixture();
    for field in 0..4 {
        let mut record = record.clone();
        let mut sealed = sealed.clone();
        match field {
            0 => record.id = DocumentId::from_uuid(Uuid::from_u128(99)),
            1 => record.version = DocumentVersion::new(4).unwrap(),
            2 => record.digest = Sha256Digest::from_array([9; 32]),
            _ => sealed.signature[0] ^= 1,
        }
        assert!(evidence
            .validate(&record, &sealed, &Hasher, &Verifier::default())
            .is_err());
    }
}

#[test]
fn independently_signed_statements_cannot_change_binding_account_or_public_material() {
    let (record, sealed, original) = fixture();
    for position in [10, 26, 62, 94, 110] {
        let mut evidence = original.clone();
        evidence.consent_statement[position] ^= 1;
        resign(&mut evidence);
        assert!(evidence
            .validate(&record, &sealed, &Hasher, &Verifier::default())
            .is_err());
    }
    for (offset, bytes) in [
        (58, 0_u32.to_be_bytes().to_vec()),
        (78, 3_u64.to_be_bytes().to_vec()),
        (86, 1_u64.to_be_bytes().to_vec()),
    ] {
        let mut evidence = original.clone();
        evidence.consent_statement[offset..offset + bytes.len()].copy_from_slice(&bytes);
        resign(&mut evidence);
        assert!(evidence
            .validate(&record, &sealed, &Hasher, &Verifier::default())
            .is_err());
    }
    let mut evidence = original;
    evidence.consent_statement[58..62].copy_from_slice(&1_u32.to_be_bytes());
    resign(&mut evidence);
    assert!(
        evidence
            .validate(&record, &sealed, &Hasher, &Verifier::default())
            .is_err(),
        "different CRLs cannot claim the same published trust revision"
    );
}

#[test]
fn altered_proofs_bad_times_versions_and_material_bounds_are_rejected() {
    let (record, sealed, original) = fixture();
    for field in 0..13 {
        let mut evidence = original.clone();
        let mut sealed = sealed.clone();
        match field {
            0 => evidence.format_version = 2,
            1 => evidence.binding_signature[0] ^= 1,
            2 => evidence.consent_signature[0] ^= 1,
            3 => evidence.binding_signature.pop().map(|_| ()).unwrap(),
            4 => evidence.consent_statement.push(0),
            5 => evidence.binding_statement.push(0),
            6 => evidence.accepted_at = 99,
            7 => evidence.accepted_at = 401,
            8 => evidence.binding_registered_at = i64::MIN,
            9 => evidence.binding_root_der = vec![0; 16 * 1024 + 1],
            10 => evidence.binding_crl_der = vec![0; 1024 * 1024 + 1],
            11 => sealed.signer_certificate_pem = vec![0; 16 * 1024 + 1],
            _ => sealed.crl_pem = vec![0; 1024 * 1024 + 1],
        }
        assert!(
            evidence
                .validate(&record, &sealed, &Hasher, &Verifier::default())
                .is_err(),
            "invalid historical evidence field {field} was accepted"
        );
    }
}

#[test]
fn untrusted_derived_fingerprints_and_verifier_metadata_are_not_accepted() {
    let (record, sealed, evidence) = fixture();
    for corruption in 1..=9 {
        let verifier = Verifier {
            corruption,
            ..Default::default()
        };
        assert!(
            evidence
                .validate(&record, &sealed, &Hasher, &verifier)
                .is_err(),
            "inconsistent verifier metadata {corruption} was accepted"
        );
    }
}

#[test]
fn unknown_duplicate_fields_and_oversized_arrays_are_rejected_when_deserializing() {
    let (_, _, evidence) = fixture();
    let mut value = serde_json::to_value(&evidence).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<IndividualSigningEvidence>(value).is_err());
    let json = serde_json::to_string(&evidence).unwrap();
    let duplicated = format!("{{\"format_version\":1,{}", &json[1..]);
    assert!(serde_json::from_str::<IndividualSigningEvidence>(&duplicated).is_err());
    for (field, size) in [
        ("binding_statement", 143),
        ("consent_statement", 259),
        ("binding_signature", 385),
        ("consent_signature", 385),
        ("binding_root_der", 16 * 1024 + 1),
        ("binding_crl_der", 1024 * 1024 + 1),
    ] {
        let mut value = serde_json::to_value(&evidence).unwrap();
        value[field] = serde_json::json!(vec![0_u8; size]);
        assert!(
            serde_json::from_value::<IndividualSigningEvidence>(value).is_err(),
            "oversized {field} must be rejected during decoding"
        );
    }
}
