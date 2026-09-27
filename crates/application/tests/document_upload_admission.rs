#[allow(dead_code)]
mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod document_upload_admission_support;

use application::{documents::*, ApplicationError};
use document_upload_admission_support::*;
use domain::identity::{Role, UserId};

#[test]
fn every_ingress_admits_exact_bytes_before_encryption_and_final_authentication() {
    let bytes = b"exact admitted bytes";
    for format in [
        AdmittedDocumentFormat::Pdf,
        AdmittedDocumentFormat::Docx,
        AdmittedDocumentFormat::Txt,
        AdmittedDocumentFormat::Jpeg,
        AdmittedDocumentFormat::Png,
        AdmittedDocumentFormat::Mp3,
        AdmittedDocumentFormat::Wav,
        AdmittedDocumentFormat::Mp4,
    ] {
        for ingress in INGRESSES {
            let mut h = Harness::new(ingress, 2);
            h.access(false);
            let trace = h.trace.clone();
            h.admission
                .expect_validate()
                .times(1)
                .withf(move |value| value == bytes)
                .return_once(move |_| {
                    trace.lock().unwrap().push("admit");
                    Ok(format)
                });
            h.commit(bytes);
            let request = h.request;
            let trace = h.trace.clone();
            let result = request.call(&h.service(), "misleading.exe", bytes).unwrap();
            assert_eq!(result.content.case_id, request.case);
            assert_eq!(result.content.document.name, "misleading.exe");
            if matches!(ingress, Ingress::Classified | Ingress::Append) {
                assert_eq!(result.current_metadata.values, metadata());
            }
            let mut expected = request.prefix();
            expected.extend(["admit", "hash", "encrypt", "authenticate", "commit"]);
            assert_eq!(*trace.lock().unwrap(), expected, "{ingress:?} {format:?}");
        }
    }
}

#[test]
fn format_rejections_keep_typed_errors_and_never_encrypt_or_persist() {
    for cause in [
        DocumentUploadError::Unsupported,
        DocumentUploadError::Invalid,
        DocumentUploadError::Limit,
        DocumentUploadError::Unavailable,
    ] {
        for ingress in INGRESSES {
            let mut h = Harness::new(ingress, 1);
            h.access(false);
            let trace = h.trace.clone();
            h.admission
                .expect_validate()
                .times(1)
                .withf(|bytes| bytes == b"rejected bytes")
                .return_once(move |_| {
                    trace.lock().unwrap().push("admit");
                    Err(cause.into())
                });
            let request = h.request;
            let trace = h.trace.clone();
            let error = request
                .call(&h.service(), "claims-to-be.pdf", b"rejected bytes")
                .unwrap_err();
            assert!(matches!(error, ApplicationError::DocumentUpload(actual) if actual == cause));
            let mut expected = request.prefix();
            expected.push("admit");
            assert_eq!(*trace.lock().unwrap(), expected, "{ingress:?} {cause:?}");
        }
    }
}

#[test]
fn invalid_sessions_and_clients_never_reach_storage_or_parser() {
    for ingress in INGRESSES {
        for revoked in [false, true] {
            let h = Harness::new(ingress, 1);
            if revoked {
                *h.state.lock().unwrap() = None;
            } else {
                h.state.lock().unwrap().as_mut().unwrap().role = Role::Client;
            }
            let request = h.request;
            let trace = h.trace.clone();
            let error = request.call(&h.service(), "a.pdf", b"bytes").unwrap_err();
            assert!(if revoked {
                matches!(error, ApplicationError::InvalidSession)
            } else {
                matches!(error, ApplicationError::PermissionDenied)
            });
            assert_eq!(*trace.lock().unwrap(), ["authenticate"]);
        }
    }
}

#[test]
fn case_access_is_checked_before_parsing_any_new_content() {
    for ingress in INGRESSES {
        let mut h = Harness::new(ingress, 1);
        h.access(true);
        let request = h.request;
        let trace = h.trace.clone();
        let error = request.call(&h.service(), "a.pdf", b"bytes").unwrap_err();
        assert!(matches!(error, ApplicationError::PermissionDenied));
        let access = if matches!(ingress, Ingress::Append) {
            "load"
        } else {
            "access"
        };
        assert_eq!(*trace.lock().unwrap(), ["authenticate", access]);
    }
}

#[test]
fn classification_permission_is_checked_before_parsing() {
    let mut h = Harness::new(Ingress::Classified, 1);
    let trace = h.trace.clone();
    h.store
        .expect_check_access()
        .times(1)
        .withf(|_, _, action| *action == DocumentAction::Upload)
        .returning(move |_, _, _| {
            trace.lock().unwrap().push("access");
            Ok(())
        });
    let trace = h.trace.clone();
    h.store
        .expect_check_access()
        .times(1)
        .withf(|_, _, action| *action == DocumentAction::Classify)
        .returning(move |_, _, _| {
            trace.lock().unwrap().push("classify");
            Err(ApplicationError::PermissionDenied)
        });
    let request = h.request;
    let trace = h.trace.clone();
    assert!(matches!(
        request.call(&h.service(), "a.pdf", b"bytes"),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(*trace.lock().unwrap(), request.prefix());
}

#[test]
fn principal_changes_during_admission_cannot_commit_under_the_original_identity() {
    for ingress in INGRESSES {
        for change in ["revoked", "id", "email", "role"] {
            let mut h = Harness::new(ingress, 2);
            h.access(false);
            let state = h.state.clone();
            let trace = h.trace.clone();
            h.admission
                .expect_validate()
                .times(1)
                .return_once(move |_| {
                    trace.lock().unwrap().push("admit");
                    let mut state = state.lock().unwrap();
                    if change == "revoked" {
                        *state = None;
                    } else {
                        let actor = state.as_mut().unwrap();
                        match change {
                            "id" => actor.id = UserId::new(),
                            "email" => actor.email = "changed@example.test".into(),
                            "role" => actor.role = Role::Litigator,
                            _ => unreachable!(),
                        }
                    }
                    Ok(AdmittedDocumentFormat::Txt)
                });
            let request = h.request;
            let trace = h.trace.clone();
            let error = request.call(&h.service(), "a.txt", b"bytes").unwrap_err();
            assert!(
                matches!(error, ApplicationError::InvalidSession),
                "{ingress:?} {change}: {error}"
            );
            let observations = trace.lock().unwrap();
            let mut prefix = request.prefix();
            prefix.push("admit");
            assert!(observations.starts_with(&prefix));
            assert_eq!(observations.last(), Some(&"authenticate"));
            assert!(!observations.contains(&"commit"));
        }
    }
}

#[test]
fn invalid_names_are_rejected_before_admission_or_encryption() {
    for ingress in INGRESSES {
        let mut h = Harness::new(ingress, 1);
        h.access(false);
        let request = h.request;
        let trace = h.trace.clone();
        assert!(matches!(
            request.call(&h.service(), "../invalid.pdf", b"bytes"),
            Err(ApplicationError::Domain(_))
        ));
        assert_eq!(*trace.lock().unwrap(), request.prefix());
    }
}

#[test]
fn the_sixteen_mib_limit_is_inclusive_and_enforced_before_the_parser() {
    assert_eq!(MAX_DOCUMENT_UPLOAD_BYTES, 16 * 1024 * 1024);
    let at_limit = vec![b'x'; MAX_DOCUMENT_UPLOAD_BYTES];
    let too_large = vec![b'x'; MAX_DOCUMENT_UPLOAD_BYTES + 1];
    for ingress in INGRESSES {
        let mut h = Harness::new(ingress, 1);
        h.access(false);
        let trace = h.trace.clone();
        h.admission
            .expect_validate()
            .times(1)
            .withf(|bytes| bytes.len() == MAX_DOCUMENT_UPLOAD_BYTES)
            .return_once(move |_| {
                trace.lock().unwrap().push("admit");
                Err(DocumentUploadError::Invalid.into())
            });
        let request = h.request;
        assert!(matches!(
            request.call(&h.service(), "a.txt", &at_limit),
            Err(ApplicationError::DocumentUpload(
                DocumentUploadError::Invalid
            ))
        ));
        let mut h = Harness::new(ingress, 1);
        h.access(false);
        let request = h.request;
        let trace = h.trace.clone();
        assert!(matches!(
            request.call(&h.service(), "a.txt", &too_large),
            Err(ApplicationError::DocumentContentTooLarge)
        ));
        assert_eq!(*trace.lock().unwrap(), request.prefix());
    }
}
