use crate::case_report_support::*;
use application::{case_reports::*, ApplicationError};
use infrastructure::{
    CaseReportEnvelopeProtector, EnvelopeKeyManager, PostgresCaseReportStore, RingAesGcmCipher,
    RingSha256Hasher,
};
use std::sync::{Arc, Mutex};
use time::{Duration, OffsetDateTime};

struct ExpiryDuringSeal {
    inner: CaseReportEnvelopeProtector,
    clock: Arc<MutableClock>,
    expires_at: Mutex<Option<OffsetDateTime>>,
}

impl CaseReportProtector for ExpiryDuringSeal {
    fn seal(
        &self,
        context: CaseReportProtectionContext,
        plaintext: &[u8],
    ) -> Result<ProtectedCaseReportPayload, ApplicationError> {
        let result = self.inner.seal(context, plaintext)?;
        if context.kind == CaseReportPayloadKind::Snapshot {
            if let Some(expiry) = self.expires_at.lock().unwrap().take() {
                self.clock.set(expiry);
            }
        }
        Ok(result)
    }

    fn open(
        &self,
        context: CaseReportProtectionContext,
        protected: &ProtectedCaseReportPayload,
    ) -> Result<Vec<u8>, ApplicationError> {
        self.inner.open(context, protected)
    }
}

#[test]
fn lease_expiry_while_sealing_capture_rolls_back_snapshot_state_and_audit() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Capture must finish before its lease", at);
    let actor = owner(&mut db);
    let timer = clock(at);
    let protector = Arc::new(ExpiryDuringSeal {
        inner: CaseReportEnvelopeProtector::new(
            Arc::new(RingAesGcmCipher::new()),
            Arc::new(EnvelopeKeyManager::new()),
            vec![0x52; 32],
        )
        .unwrap(),
        clock: timer.clone(),
        expires_at: Mutex::new(None),
    });
    let store = PostgresCaseReportStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        timer,
        protector.clone(),
    )
    .unwrap();
    request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    let before = ledger(&mut db);
    *protector.expires_at.lock().unwrap() = Some(claim.lease.expires_at);

    let result = store.capture(&claim.lease, at);
    assert!(
        matches!(
            result,
            Err(ApplicationError::CaseReport(CaseReportError::LeaseLost))
        ),
        "capture accepted a lease which expired during encryption: {result:?}"
    );
    assert_eq!(
        ledger(&mut db),
        before,
        "expired capture left durable writes"
    );
    assert_eq!(count(&mut db, "case_report_snapshots"), 0);
    assert_eq!(audits(&mut db, "case_report.capture"), 0);
}

#[test]
fn corrupted_interrupted_capture_fails_audited_without_blocking_the_next_report() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "A later report must remain claimable", at);
    let actor = owner(&mut db);
    let timer = clock(at);
    let initial = store(&db, timer.clone());
    let a = request(&initial, &actor, command(at), at).unwrap();
    let b = request(&initial, &actor, command(at), at).unwrap();
    let claim = initial.claim_next(at).unwrap().unwrap();
    let broken_id = claim.lease.report_id;
    assert_eq!(broken_id, a.id.min(b.id));
    let healthy_id = a.id.max(b.id);
    initial.capture(&claim.lease, at).unwrap();
    drop(initial);

    // Model corrupted persisted ciphertext while restoring the immutable guard
    // before opening the runtime store. Length, identity and grants stay valid.
    db.admin
        .batch_execute(
            "ALTER TABLE case_report_snapshots DISABLE TRIGGER case_report_snapshot_immutable",
        )
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_report_snapshots SET payload=set_byte(payload,31,get_byte(payload,31)#1)
            WHERE report_id=$1",
            &[&broken_id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute(
            "ALTER TABLE case_report_snapshots ENABLE TRIGGER case_report_snapshot_immutable",
        )
        .unwrap();
    let original_payload: Vec<u8> = db
        .admin
        .query_one(
            "SELECT payload FROM case_report_snapshots WHERE report_id=$1",
            &[&broken_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    let later = claim.lease.expires_at + Duration::seconds(1);
    timer.set(later);
    let resumed = store(&db, timer);

    let next = resumed
        .claim_next(later)
        .expect("corrupted capture must be recorded as a terminal failure");
    let failed = resumed.get(&actor, broken_id, later).unwrap();
    assert_eq!(
        failed.state,
        CaseReportState::Failed(CaseReportFailure::InvalidStoredCapture)
    );
    let notice = failed
        .notice
        .expect("failed report requires its durable notice");
    assert_eq!(notice.kind, CaseReportNoticeKind::Failed);
    assert_eq!(notice.created_at, later);
    assert_eq!(notice.read_at, None);
    assert_eq!(audits(&mut db, "case_report.fail"), 1);
    assert_eq!(count(&mut db, "case_report_artifacts"), 0);
    assert_eq!(count(&mut db, "case_report_notices"), 1);
    let retained_payload: Vec<u8> = db
        .admin
        .query_one(
            "SELECT payload FROM case_report_snapshots WHERE report_id=$1",
            &[&broken_id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        retained_payload, original_payload,
        "failure replaced corrupted evidence"
    );

    let next = next
        .or_else(|| resumed.claim_next(later).unwrap())
        .expect("healthy report remains queued");
    assert_eq!(next.report.id, healthy_id);
    assert_eq!(next.lease.generation, 1);
    assert_eq!(next.snapshot, None);
}
