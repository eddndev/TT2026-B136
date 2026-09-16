use super::{projection, route_support::*};
use application::typed_participants::*;
use axum::body::Body;
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{clock::OffsetDateTime, crypto::*};
use std::sync::Arc;

#[tokio::test]
async fn credential_download_preserves_public_bytes_trust_and_distinct_times() {
    let w = Arc::new(Workflow::default());
    let subject = subject_snapshot();
    let digest = Sha256Digest::from_array([0x77; 32]);
    let trust = CredentialTrustInspection {
        root_der: vec![1, 2, 3],
        crl_der: vec![4, 5, 6],
        root_fingerprint: digest,
        crl_digest: digest,
        crl_number: u64::MAX,
        crl_this_update: -10,
        crl_next_update: 100,
        valid_from: -5,
        valid_until: 95,
    };
    let certificate = CredentialCertificate {
        der: vec![7, 8, 9],
        fingerprint: digest,
        summary: CertificateSummary {
            subject: "CN=Public subject".into(),
            issuer: "CN=Internal authority".into(),
            serial_hex: "AA".into(),
            not_before_unix: -1,
            not_after_unix: 90,
        },
    };
    let reference = ParticipantCredentialRef {
        participant_id: ParticipantId::from_uuid(Uuid::parse_str(PARTICIPANT).unwrap()),
        participant_revision: ParticipantRevision::initial(),
        statement_digest: digest,
    };
    *w.evidence.lock().unwrap() = Some(ParticipantCredentialEvidence {
        case_id: case_id(),
        reference,
        subject: prepared().proposal.values().subject(),
        declaration: vec![0x11; 218],
        check: CredentialCheck {
            certificate,
            trust: trust.clone(),
            statement_digest: digest,
            signature: Signature::from_bytes(vec![0x22; 384]).unwrap(),
            checked_at: 5,
            valid_from: -1,
            valid_until: 90,
        },
        trust: CredentialTrustSnapshot {
            deployment_id: Uuid::from_u128(99),
            revision: CredentialTrustRevision::initial(),
            inspection: trust,
            published_at: OffsetDateTime::UNIX_EPOCH,
            published_by: "database-publisher".into(),
        },
        accepted_at: OffsetDateTime::from_unix_timestamp(7).unwrap(),
        accepted_by: subject.changed_by,
    });
    let response = request(
        &w,
        "GET",
        &format!("participants/{PARTICIPANT}/revisions/1/credential"),
        Some("reader"),
        Body::empty(),
    )
    .await;
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let value = body(response).await;
    assert_eq!(value["trust"]["crl_number"], u64::MAX.to_string());
    assert_eq!(value["checked_at_unix"], 5);
    assert_eq!(value["accepted_at"], "1970-01-01T00:00:07Z");
    assert_eq!(value["trust"]["crl_this_update_unix"], -10);
    assert_eq!(value["trust"]["crl_next_update_unix"], 100);
    assert_eq!(
        STANDARD
            .decode(value["declaration_base64"].as_str().unwrap())
            .unwrap(),
        vec![0x11; 218]
    );
    assert_eq!(
        STANDARD
            .decode(value["signature_base64"].as_str().unwrap())
            .unwrap(),
        vec![0x22; 384]
    );
    assert_eq!(value["certificate_summary"]["serial_hex"], "AA");
    assert_eq!(value["policy"], "internal_demo_v1");
    assert!(!value.to_string().contains("private_key"));
}

#[test]
fn status_revision_keeps_credential_origin_and_exact_subject_binding() {
    let mut row = detail();
    let ParticipantRevisionSnapshot::Typed(value) = &mut row.revision else {
        panic!("typed fixture")
    };
    value.revision = ParticipantRevision::new(2).unwrap();
    value.values = value
        .values
        .with_directory_status(DirectoryStatus::Archived);
    value.credential_origin = Some(ParticipantCredentialRef {
        participant_id: value.id,
        participant_revision: ParticipantRevision::initial(),
        statement_digest: Sha256Digest::from_array([0x77; 32]),
    });
    let projected = projection::detail(row.clone()).unwrap();
    assert_eq!(projected["revision"], 2);
    assert_eq!(projected["credential_origin"]["participant_revision"], 1);
    assert_eq!(projected["submission_revision"], 1);
    assert_eq!(projected["directory_status"], "archived");
    row.bound_subject.as_mut().unwrap().revision = SubjectRevision::new(2).unwrap();
    assert!(projection::detail(row).is_err());
}
