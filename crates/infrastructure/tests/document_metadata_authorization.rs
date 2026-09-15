#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{
    CaseDocumentStore, DocumentMetadata, MetadataQuery, MetadataRevision,
};
use application::ApplicationError;
use domain::identity::UserId;
use metadata_database_support::{document, metadata, Fixture};

#[test]
fn metadata_operations_recheck_roles_membership_and_exact_document_case() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
    for role in ["litigator", "paralegal", "client"] {
        let user = UserId::new();
        f.db.client.execute("INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes) VALUES($1,$2,'fixture',$3,'\\x00','{}')",&[&user.as_uuid(),&format!("{user}@example.test"),&role]).unwrap();
        f.db.client
            .execute(
                "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                &[&f.case.as_uuid(), &user.as_uuid()],
            )
            .unwrap();
        if role == "client" {
            let before = f.snapshot();
            for result in operations(&store, user, f.case, record.id, f.at) {
                assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            }
            assert!(matches!(
                store.insert_with_metadata(
                    user,
                    f.case,
                    document(),
                    DocumentMetadata::empty(),
                    f.at
                ),
                Err(ApplicationError::PermissionDenied)
            ));
            assert_eq!(f.snapshot(), before);
        } else {
            let current = store.get_metadata(user, f.case, record.id, f.at).unwrap();
            store
                .replace_metadata(
                    user,
                    f.case,
                    record.id,
                    current.metadata_revision,
                    metadata(role, "Class", &[]),
                    f.at,
                )
                .unwrap();
            f.db.client
                .execute(
                    "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                    &[&f.case.as_uuid(), &user.as_uuid()],
                )
                .unwrap();
            let before = f.snapshot();
            for result in operations(&store, user, f.case, record.id, f.at) {
                assert!(matches!(result, Err(ApplicationError::DocumentNotFound(_))));
            }
            assert_eq!(f.snapshot(), before);
        }
    }
    let foreign_case = domain::cases::CaseId::from_uuid(f.db.seed_case());
    let before = f.snapshot();
    for id in [record.id, domain::crypto::DocumentId::new()] {
        for result in operations(&store, f.owner, foreign_case, id, f.at) {
            assert!(matches!(result, Err(ApplicationError::DocumentNotFound(_))));
        }
    }
    assert_eq!(f.snapshot(), before);
    f.db.client
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&f.owner.as_uuid()],
        )
        .unwrap();
    for result in operations(&store, f.owner, f.case, record.id, f.at) {
        assert!(matches!(result, Err(ApplicationError::InvalidSession)));
    }
    assert_eq!(f.snapshot(), before);
}

fn operations(
    store: &infrastructure::PostgresCaseDocumentStore,
    actor: UserId,
    case: domain::cases::CaseId,
    id: domain::crypto::DocumentId,
    at: time::OffsetDateTime,
) -> [Result<(), ApplicationError>; 4] {
    [
        store.get_metadata(actor, case, id, at).map(|_| ()),
        store
            .metadata_history(actor, case, id, MetadataQuery::new(10, None).unwrap(), at)
            .map(|_| ()),
        store.get_overview(actor, case, id, at).map(|_| ()),
        store
            .replace_metadata(
                actor,
                case,
                id,
                MetadataRevision::unclassified(),
                DocumentMetadata::empty(),
                at,
            )
            .map(|_| ()),
    ]
}
