use crate::case_report_support::*;
use application::cases::CaseRepository;
use application::{
    case_reports::*,
    identity::{Principal, UserRecord, UserRepository},
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{PasswordHasher, RecoveryCodeOutcome, RecoveryCodeSet, RECOVERY_CODE_COUNT},
    identity::{Role, UserId},
};
use infrastructure::{Argon2idHasher, PostgresUserRepository};

fn enrolled_litigator(db: &mut Fixture) -> (PostgresUserRepository, Principal) {
    let at = db.at;
    let case = seed_case(db, "Recovery-access report", at);
    let repository = PostgresUserRepository::open(&db.runtime_url).unwrap();
    let id = UserId::new();
    let hash = Argon2idHasher::new()
        .hash("report-recovery-fixture")
        .unwrap();
    let user = UserRecord {
        id,
        email: format!("{id}@example.test"),
        password_hash: "fixture".into(),
        role: Role::Litigator,
        active: true,
        protected_totp_secret: vec![0; 48],
        recovery_codes: RecoveryCodeSet::from_hashes(vec![hash; RECOVERY_CODE_COUNT]).unwrap(),
        revision: 0,
        auth_generation: 0,
    };
    repository.insert(user, db.owner, at).unwrap();
    db.store().add_member(case, id, db.owner, at).unwrap();
    (repository, principal(db, id))
}
fn consume(repository: &PostgresUserRepository, actor: &Principal, at: OffsetDateTime) {
    let before = repository.find_by_id(actor.id).unwrap().unwrap();
    let mut codes = before.recovery_codes.clone();
    assert_eq!(
        codes
            .consume("report-recovery-fixture", &Argon2idHasher::new())
            .unwrap(),
        RecoveryCodeOutcome::Accepted
    );
    repository
        .replace_recovery_codes(actor.id, before.revision, codes, at)
        .unwrap();
    let after = repository.find_by_id(actor.id).unwrap().unwrap();
    assert_eq!(
        after.recovery_codes.remaining(),
        before.recovery_codes.remaining() - 1
    );
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.auth_generation, before.auth_generation);
}

#[test]
fn recovery_consumption_preserves_ready_exports_replay_and_captured_worker_access() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let (users, actor) = enrolled_litigator(&mut db);
    let store = store(&db, clock(at));
    let original = command(at);
    let ready = finish(&store, &actor, original.clone(), at);
    let queued = request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    assert_eq!(claim.report.id, queued.id);
    let snapshot = store.capture(&claim.lease, at).unwrap();
    consume(&users, &actor, at);
    assert_eq!(audits(&mut db, "identity.recovery_consumed"), 1);
    assert_eq!(store.get(&actor, ready.id, at).unwrap(), ready);
    assert_eq!(request(&store, &actor, original, at).unwrap(), ready);
    assert_eq!(
        store
            .download(&actor, ready.id, CaseReportFormat::Pdf, at)
            .unwrap()
            .report,
        ready
    );
    assert_eq!(store.list(&actor, query(), at).unwrap().reports.len(), 2);
    let renewed = store.renew(&claim.lease, at).unwrap();
    let completed = store
        .complete(&renewed, &snapshot, artifacts(&snapshot), at)
        .unwrap();
    assert!(matches!(completed.state, CaseReportState::Ready { .. }));
    assert_eq!(completed.requester, queued.requester);
    let acknowledged = store.acknowledge_notice(&actor, ready.id, at).unwrap();
    assert_eq!(acknowledged.notice.unwrap().read_at, Some(at));
}

#[test]
fn recovery_revision_advance_cannot_mask_access_generation_change_and_restoration() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let (users, actor) = enrolled_litigator(&mut db);
    let store = store(&db, clock(at));
    let command = command(at);
    let ready = finish(&store, &actor, command.clone(), at);
    consume(&users, &actor, at);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&actor.id.as_uuid()]).unwrap();
    db.admin.execute("UPDATE users SET active=true,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&actor.id.as_uuid()]).unwrap();
    assert!(matches!(
        store.get(&actor, ready.id, at),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
    assert!(matches!(
        request(&store, &actor, command, at),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
    assert!(matches!(
        store.download(&actor, ready.id, CaseReportFormat::Csv, at),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
    assert!(store.list(&actor, query(), at).unwrap().reports.is_empty());
}
