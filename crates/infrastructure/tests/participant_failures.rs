#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    DirectoryStatus, ParticipantHistoryQuery, ParticipantId, ParticipantQuery, ParticipantRevision,
    ParticipantStatusFilter, ParticipantStore,
};
use application::ApplicationError;
use participant_database_support::{values, Fixture};

#[test]
fn row_audit_and_deferred_commit_failures_roll_back_complete_mutations() {
    for operation in ["create", "replace", "status"] {
        for failure in ["row", "audit", "commit"] {
            let Some(mut f) = Fixture::new() else { return };
            let store = f.store();
            let id = ParticipantId::new();
            if operation != "create" {
                store
                    .create(f.owner, f.case, id, values("Before"), f.at)
                    .unwrap();
            }
            if failure == "audit" {
                f.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_participant_audit CHECK(action NOT LIKE 'participant.%') NOT VALID").unwrap();
            } else {
                let (constraint, timing, deferred) = if failure == "commit" {
                    ("CONSTRAINT ", "AFTER", "DEFERRABLE INITIALLY DEFERRED")
                } else {
                    ("", "BEFORE", "")
                };
                f.admin.batch_execute(&format!("CREATE FUNCTION reject_participant() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected participant failure'; END $$; CREATE {constraint}TRIGGER reject_participant {timing} INSERT ON case_participant_revisions {deferred} FOR EACH ROW EXECUTE FUNCTION reject_participant()")).unwrap();
            }
            let before = f.snapshot();
            let result = match operation {
                "create" => store
                    .create(f.owner, f.case, id, values("New"), f.at)
                    .map(|_| ()),
                "replace" => store
                    .replace(
                        f.owner,
                        f.case,
                        id,
                        ParticipantRevision::initial(),
                        values("Changed"),
                        f.at,
                    )
                    .map(|_| ()),
                _ => store
                    .change_status(
                        f.owner,
                        f.case,
                        id,
                        ParticipantRevision::initial(),
                        DirectoryStatus::Archived,
                        f.at,
                    )
                    .map(|_| ()),
            };
            assert!(
                matches!(result, Err(ApplicationError::Port(_))),
                "{operation}/{failure}: {result:?}"
            );
            assert_eq!(f.snapshot(), before, "{operation}/{failure}");
        }
    }
}

#[test]
fn failed_read_audit_returns_no_participant_data() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Private"), f.at)
        .unwrap();
    f.admin.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_participant_reads CHECK(action NOT IN ('participant.read','participant.listed','participant.history_listed'))").unwrap();
    let before = f.snapshot();
    let results = [
        store.get(f.owner, f.case, id, f.at).map(|_| ()),
        store
            .history(
                f.owner,
                f.case,
                id,
                ParticipantHistoryQuery::new(10, None).unwrap(),
                f.at,
            )
            .map(|_| ()),
        store
            .list(
                f.owner,
                f.case,
                ParticipantQuery::new(10, None, None, None, ParticipantStatusFilter::All).unwrap(),
                f.at,
            )
            .map(|_| ()),
    ];
    assert!(results
        .into_iter()
        .all(|r| matches!(r, Err(ApplicationError::Port(_)))));
    assert_eq!(f.snapshot(), before);
}

#[test]
fn persisted_corruption_is_not_normalized_into_a_valid_response() {
    for corruption in [
        "display_name=' Bad'",
        "organization=''",
        "values_digest='\\x00'",
        "values_digest=decode(repeat('00',32),'hex')",
        "changed_at='2025-01-01T01:00:00+01:00'",
        "changed_by_email=''",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let id = ParticipantId::new();
        store
            .create(f.owner, f.case, id, values("Valid"), f.at)
            .unwrap();
        f.admin.batch_execute("ALTER TABLE case_participant_revisions DISABLE TRIGGER participant_revision_immutable; ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_canonical; ALTER TABLE case_participant_revisions DROP CONSTRAINT participant_digest").unwrap();
        f.admin
            .batch_execute(&format!(
                "UPDATE case_participant_revisions SET {corruption}"
            ))
            .unwrap();
        let before = f.snapshot();
        for result in [
            store.get(f.owner, f.case, id, f.at).map(|_| ()),
            store
                .history(
                    f.owner,
                    f.case,
                    id,
                    ParticipantHistoryQuery::new(10, None).unwrap(),
                    f.at,
                )
                .map(|_| ()),
            store
                .replace(
                    f.owner,
                    f.case,
                    id,
                    ParticipantRevision::initial(),
                    values("Repair"),
                    f.at,
                )
                .map(|_| ()),
            store
                .list(
                    f.owner,
                    f.case,
                    ParticipantQuery::new(10, None, None, None, ParticipantStatusFilter::All)
                        .unwrap(),
                    f.at,
                )
                .map(|_| ()),
        ] {
            assert!(
                matches!(
                    result,
                    Err(ApplicationError::StoredParticipantInconsistent(_))
                ),
                "accepted {corruption}: {result:?}"
            );
        }
        assert_eq!(f.snapshot(), before);
    }
}
