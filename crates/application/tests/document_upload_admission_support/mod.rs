mod probe;

use std::sync::{Arc, Mutex};

use application::documents::*;
use application::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion},
    identity::{Role, UserId},
};
use mockall::mock;

use crate::case_document_support::{MockIdentity, MockStore};

pub type Trace = Arc<Mutex<Vec<&'static str>>>;
pub type IdentityState = Arc<Mutex<Option<Principal>>>;

mock! {
    pub Admission {}
    impl DocumentUploadAdmission for Admission {
        fn validate(&self, bytes: &[u8]) -> Result<AdmittedDocumentFormat, ApplicationError>;
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Ingress {
    Upload,
    Classified,
    Append,
}
pub const INGRESSES: [Ingress; 3] = [Ingress::Upload, Ingress::Classified, Ingress::Append];

#[derive(Clone, Copy)]
pub struct Request {
    pub ingress: Ingress,
    pub case: CaseId,
    pub id: DocumentId,
}
impl Request {
    pub fn call(
        self,
        service: &CaseDocumentService,
        name: &str,
        bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
        match self.ingress {
            Ingress::Upload => service.upload("session", self.case, name, bytes),
            Ingress::Classified => {
                service.upload_with_metadata("session", self.case, name, bytes, metadata())
            }
            Ingress::Append => service.append(
                "session",
                self.case,
                self.id,
                DocumentVersion::initial(),
                name,
                bytes,
            ),
        }
    }
    pub fn prefix(self) -> Vec<&'static str> {
        match self.ingress {
            Ingress::Upload => vec!["authenticate", "access"],
            Ingress::Classified => vec!["authenticate", "access", "classify"],
            Ingress::Append => vec!["authenticate", "load"],
        }
    }
}

pub fn metadata() -> DocumentMetadata {
    DocumentMetadata::new(Some("Escrito"), Some("Publico"), &["Exacto".into()]).unwrap()
}

pub struct Harness {
    pub request: Request,
    pub principal: Principal,
    pub state: IdentityState,
    pub trace: Trace,
    pub store: MockStore,
    pub identity: MockIdentity,
    pub admission: MockAdmission,
    old: DocumentRecord,
}
impl Harness {
    pub fn new(ingress: Ingress, authentication_calls: usize) -> Self {
        let principal = Principal {
            id: UserId::new(),
            email: "operator@example.test".into(),
            role: Role::Owner,
        };
        let state = Arc::new(Mutex::new(Some(principal.clone())));
        let trace = Arc::new(Mutex::new(Vec::new()));
        let mut identity = MockIdentity::new();
        let identity_state = state.clone();
        let identity_trace = trace.clone();
        identity
            .expect_authenticate()
            .times(authentication_calls)
            .withf(|token| token == "session")
            .returning(move |_| {
                identity_trace.lock().unwrap().push("authenticate");
                identity_state
                    .lock()
                    .unwrap()
                    .clone()
                    .ok_or(ApplicationError::InvalidSession)
            });
        let old = crate::crypto::processor()
            .prepare("historical.bin", b"old bytes")
            .unwrap();
        Self {
            request: Request {
                ingress,
                case: CaseId::new(),
                id: old.id,
            },
            principal,
            state,
            trace,
            identity,
            old,
            store: MockStore::new(),
            admission: MockAdmission::new(),
        }
    }

    pub fn access(&mut self, deny: bool) {
        let actor = self.principal.id;
        let request = self.request;
        match request.ingress {
            Ingress::Append => {
                let old = self.old.clone();
                let trace = self.trace.clone();
                self.store
                    .expect_load()
                    .times(1)
                    .withf(move |who, case, id, selection, action| {
                        *who == actor
                            && *case == request.case
                            && *id == request.id
                            && *selection == VersionSelection::Current
                            && *action == DocumentAction::Append
                    })
                    .return_once(move |_, _, _, _, _| {
                        trace.lock().unwrap().push("load");
                        if deny {
                            Err(ApplicationError::PermissionDenied)
                        } else {
                            Ok(old)
                        }
                    });
            }
            Ingress::Upload | Ingress::Classified => {
                let actions: &[DocumentAction] = if matches!(request.ingress, Ingress::Classified) {
                    &[DocumentAction::Upload, DocumentAction::Classify]
                } else {
                    &[DocumentAction::Upload]
                };
                for &action in actions {
                    let trace = self.trace.clone();
                    self.store
                        .expect_check_access()
                        .times(1)
                        .withf(move |who, case, selected| {
                            *who == actor && *case == request.case && *selected == action
                        })
                        .returning(move |_, _, _| {
                            trace
                                .lock()
                                .unwrap()
                                .push(if action == DocumentAction::Classify {
                                    "classify"
                                } else {
                                    "access"
                                });
                            if deny {
                                Err(ApplicationError::PermissionDenied)
                            } else {
                                Ok(())
                            }
                        });
                    if deny {
                        break;
                    }
                }
            }
        }
    }

    pub fn commit(&mut self, expected_bytes: &[u8]) {
        let actor = self.principal.id;
        let request = self.request;
        let trace = self.trace.clone();
        let expected_bytes = expected_bytes.to_vec();
        let verify = move |case, record: DocumentRecord, values| {
            assert_eq!(
                crate::crypto::processor()
                    .content_plaintext(&record)
                    .unwrap()
                    .as_slice(),
                expected_bytes.as_slice()
            );
            trace.lock().unwrap().push("commit");
            Ok(DocumentOverview {
                content: CaseDocumentSummary {
                    case_id: case,
                    document: DocumentSummary::from(&record),
                },
                current_metadata: CurrentDocumentMetadata {
                    metadata_revision: MetadataRevision::new(1),
                    values,
                },
            })
        };
        match request.ingress {
            Ingress::Upload => {
                self.store
                    .expect_insert()
                    .times(1)
                    .withf(move |who, case, record, _| {
                        *who == actor
                            && *case == request.case
                            && record.version == DocumentVersion::initial()
                    })
                    .return_once(move |_, case, record, _| {
                        verify(case, record, DocumentMetadata::empty())
                    });
            }
            Ingress::Classified => {
                self.store
                    .expect_insert_with_metadata()
                    .times(1)
                    .withf(move |who, case, record, values, _| {
                        *who == actor
                            && *case == request.case
                            && *values == metadata()
                            && record.version == DocumentVersion::initial()
                    })
                    .return_once(move |_, case, record, values, _| verify(case, record, values));
            }
            Ingress::Append => {
                self.store
                    .expect_append()
                    .times(1)
                    .withf(move |who, case, expected, record, _| {
                        *who == actor
                            && *case == request.case
                            && *expected == DocumentVersion::initial()
                            && record.id == request.id
                            && record.version.get() == 2
                    })
                    .return_once(move |_, case, _, record, _| verify(case, record, metadata()));
            }
        }
    }

    pub fn service(self) -> CaseDocumentService {
        CaseDocumentService::new(
            Arc::new(self.store),
            Arc::new(self.identity),
            probe::processor(self.trace),
            Arc::new(self.admission),
            Arc::new(crate::crypto::TestClock),
        )
    }
}
