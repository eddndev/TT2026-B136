use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustStore},
    identity::Principal,
    typed_participants::*,
};
use der::{
    asn1::{OctetString, Uint},
    oid::AssociatedOid,
    Encode,
};
use domain::{clock::Clock, crypto::*, identity::Role};
use infrastructure::{
    certificates::InternalRsaDeclarationVerifier, PostgresCredentialTrustStore,
    PostgresTypedParticipantStore, RingSha256Hasher,
};
use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc, Mutex,
};
use time::OffsetDateTime;
use x509_cert::ext::pkix::CrlNumber;

use crate::typed_participant_service_support::*;

pub struct MovingClock(pub AtomicI64);
impl Clock for MovingClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst))
            .unwrap()
            .replace_nanosecond(123456789)
            .unwrap()
    }
}

#[derive(Default)]
pub struct CapturingVerifier(pub Mutex<Option<CredentialCheck>>);
impl InternalDeclarationVerifier for CapturingVerifier {
    fn inspect_certificate(
        &self,
        bytes: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        InternalRsaDeclarationVerifier.inspect_certificate(bytes)
    }
    fn inspect_trust(
        &self,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialTrustInspection, CredentialFailure> {
        InternalRsaDeclarationVerifier.inspect_trust(root, crl, at)
    }
    fn verify(
        &self,
        digest: &Sha256Digest,
        certificate: &[u8],
        signature: &Signature,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialCheck, CredentialFailure> {
        let check =
            InternalRsaDeclarationVerifier.verify(digest, certificate, signature, root, crl, at)?;
        *self.0.lock().unwrap() = Some(check.clone());
        Ok(check)
    }
}

pub struct Scenario {
    pub db: Fixture,
    pub clock: Arc<MovingClock>,
    pub request: ParticipantPreparationRequest,
    pub signature: Signature,
    pub trust: CredentialTrustSnapshot,
}
impl Scenario {
    pub fn new() -> Option<Self> {
        let mut db = Fixture::new()?;
        let fx = crate::declaration_fixture::fixture();
        let clock = Arc::new(MovingClock(AtomicI64::new(fx.at)));
        db.at = clock.now();
        let trust = PostgresCredentialTrustStore::open(&db.admin_url, clock.clone())
            .unwrap()
            .publish(CredentialTrustExpectation::Absent, inspection(1, -100))
            .unwrap();
        let record = upload(&db, db.case, "identity.pdf");
        let workflow = service(&db, FormatCheck(None));
        let mut proposal = proposal(&record);
        proposal.certificate = Some(fx.leaf.clone());
        proposal.role = ParticipantRoleValues::new(
            None,
            None,
            ParticipantProfile::ControlJudge(ControlJudgeProfile::new("Declared court").unwrap()),
            proposal.role.role_support().clone(),
        )
        .unwrap();
        let mut request = reviewed(
            workflow
                .review_participant("session", db.case, proposal)
                .unwrap(),
        );
        request.certificate = Some(fx.leaf.clone());
        let declaration = workflow
            .prepare_participant("session", db.case, request.clone())
            .unwrap()
            .declaration
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("statement.bin");
        std::fs::write(&path, &declaration.bytes).unwrap();
        let signature = Signature::from_bytes(crate::declaration_fixture::openssl(&[
            "dgst",
            "-sha256",
            "-sign",
            fx.ca
                .join("private/synthetic-declarant.key.pem")
                .to_str()
                .unwrap(),
            path.to_str().unwrap(),
        ]))
        .unwrap();
        Some(Self {
            db,
            clock,
            request,
            signature,
            trust,
        })
    }

    pub fn workflow(
        &self,
        format: FormatCheck,
        verifier: Arc<CapturingVerifier>,
        application_name: &str,
    ) -> TypedParticipantService {
        let mut url = reqwest::Url::parse(&self.db.runtime_url).unwrap();
        url.query_pairs_mut()
            .append_pair("application_name", application_name);
        let store = PostgresTypedParticipantStore::open(
            url.as_str(),
            Arc::new(RingSha256Hasher),
            self.clock.clone(),
        )
        .unwrap();
        TypedParticipantService::new(
            Arc::new(TestIdentity(Principal {
                id: self.db.owner,
                email: "session@example.test".into(),
                role: Role::Owner,
            })),
            Arc::new(store),
            processor(),
            Arc::new(RingSha256Hasher),
            Arc::new(format),
            verifier,
            self.clock.clone(),
        )
    }

    pub fn submission(&self) -> ParticipantSubmission {
        ParticipantSubmission {
            prepared: self.request.clone(),
            signature: Some(self.signature.clone()),
        }
    }
}

pub fn inspection(number: u64, start: i64) -> CredentialTrustInspection {
    let fx = crate::declaration_fixture::fixture();
    let mut crl = crate::declaration_fixture::crl();
    let number = CrlNumber(Uint::new(&number.to_be_bytes()).unwrap());
    let extension = crl
        .tbs_cert_list
        .crl_extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v.extn_id == CrlNumber::OID)
        .unwrap();
    extension.extn_value = OctetString::new(number.to_der().unwrap()).unwrap();
    crl.tbs_cert_list.this_update = crate::declaration_fixture::time(fx.at + start);
    crl.tbs_cert_list.next_update = Some(crate::declaration_fixture::time(fx.at + 100));
    InternalRsaDeclarationVerifier
        .inspect_trust(
            &fx.root,
            &crate::declaration_fixture::signed_crl(crl),
            fx.at,
        )
        .unwrap()
}

pub fn audit(db: &mut Fixture) -> Vec<serde_json::Value> {
    db.admin
        .query(
            "SELECT to_jsonb(a) FROM audit_events a ORDER BY sequence",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

pub fn no_participant_changes(db: &mut Fixture) {
    for table in [
        "case_participants",
        "case_subjects",
        "case_subject_revisions",
        "case_participant_typed_revisions",
        "subject_identity_reviews",
        "participant_identity_reviews",
        "participant_credential_evidence",
    ] {
        let count: i64 = db
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "{table}");
    }
}
