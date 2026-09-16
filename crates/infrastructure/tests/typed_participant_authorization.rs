mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod typed_participant_service_support;
use application::{identity::Principal, typed_participants::*, ApplicationError};
use domain::identity::{Role, UserId};
use std::sync::Arc;
use typed_participant_service_support::*;
fn workflow(
    db: &Fixture,
    actor: UserId,
    role: Role,
    format: FormatCheck,
) -> TypedParticipantService {
    TypedParticipantService::new(
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        store(db),
        processor(),
        Arc::new(infrastructure::RingSha256Hasher),
        Arc::new(format),
        Arc::new(infrastructure::certificates::InternalRsaDeclarationVerifier),
        Arc::new(FixedClock(db.at)),
    )
}
#[test]
fn all_roles_use_current_membership_and_scope_for_identity_reads_and_management() {
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let owner = service(&db, FormatCheck(None));
    let request = reviewed(
        owner
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    let result = owner
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        )
        .unwrap();
    let subject = result.bound_subject.unwrap();
    for (role, name, read, manage) in [
        (Role::Owner, "owner", true, true),
        (Role::Litigator, "litigator", true, true),
        (Role::Paralegal, "paralegal", true, false),
        (Role::Client, "client", false, false),
    ] {
        let actor = db.user(name, true);
        let operations = workflow(&db, actor, role, FormatCheck(None));
        assert_eq!(
            operations
                .get_subject("session", db.case, subject.id)
                .is_ok(),
            read
        );
        assert_eq!(
            operations
                .review_subject(
                    "session",
                    db.case,
                    subject.id,
                    subject.revision,
                    subject.values.clone()
                )
                .is_ok(),
            manage
        );
        db.admin
            .execute(
                "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                &[&db.case.as_uuid(), &actor.as_uuid()],
            )
            .unwrap();
        assert_eq!(
            operations
                .get_subject("session", db.case, subject.id)
                .is_ok(),
            role == Role::Owner
        );
    }
    let foreign = domain::cases::CaseId::new();
    assert!(matches!(
        store(&db).get_subject(db.owner, foreign, subject.id, db.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        store(&db).get_subject(db.owner, db.case, CaseSubjectId::new(), db.at),
        Err(ApplicationError::SubjectNotFound)
    ));
}
#[test]
fn membership_revocation_and_account_disable_during_parser_prevent_commit() {
    for disable in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let record = upload(&db, db.case, "support.pdf");
        let actor = db.user("litigator", true);
        let ops = workflow(&db, actor, Role::Litigator, FormatCheck(None));
        let request = reviewed(
            ops.review_participant("session", db.case, proposal(&record))
                .unwrap(),
        );
        let url = db.admin_url.clone();
        let case = db.case;
        let format = FormatCheck(Some(Box::new(move || {
            let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
            if disable {
                client
                    .execute(
                        "UPDATE users SET active=FALSE WHERE id=$1",
                        &[&actor.as_uuid()],
                    )
                    .unwrap();
            } else {
                client
                    .execute(
                        "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                        &[&case.as_uuid(), &actor.as_uuid()],
                    )
                    .unwrap();
            }
        })));
        let result = workflow(&db, actor, Role::Litigator, format).submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        );
        assert!(result.is_err(), "{result:?}");
        assert_eq!(
            db.admin
                .query_one("SELECT count(*) FROM case_participant_typed_revisions", &[])
                .unwrap()
                .get::<_, i64>(0),
            0
        );
        assert_eq!(
            db.admin
                .query_one(
                    "SELECT count(*) FROM audit_events WHERE action='participant.typed_created'",
                    &[]
                )
                .unwrap()
                .get::<_, i64>(0),
            0
        );
    }
}
#[test]
fn closed_administration_blocks_new_typed_changes_after_preparation() {
    use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
    let Some(mut db) = Fixture::new() else { return };
    let record = upload(&db, db.case, "support.pdf");
    let request = reviewed(
        service(&db, FormatCheck(None))
            .review_participant("session", db.case, proposal(&record))
            .unwrap(),
    );
    let admin = infrastructure::PostgresCaseRepository::connect(
        &db.admin_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    let (case, owner, at) = (db.case, db.owner, db.at);
    let format = FormatCheck(Some(Box::new(move || {
        admin
            .change_administrative_status(
                owner,
                case,
                CaseRevisionExpectation::Unrevised,
                CaseAdministrativeStatus::Closed,
                at,
            )
            .unwrap();
    })));
    let result = service(&db, format).submit_participant(
        "session",
        db.case,
        ParticipantSubmission {
            prepared: request,
            signature: None,
        },
    );
    assert!(
        matches!(result, Err(ApplicationError::CaseClosed)),
        "{result:?}"
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_subjects", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}
