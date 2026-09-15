mod case_administration_support;
use application::cases::*;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::cases::{CaseId, CaseMetadata};

fn profile(nuc: &str) -> PenalCaseProfile {
    PenalCaseProfile::new(nuc, "Office", nuc, "Court", &["Reported"], None, None).unwrap()
}
fn query(
    status: CaseStatusFilter,
    profile: CaseProfileFilter,
    title: Option<&str>,
    nuc: Option<&str>,
) -> CaseAdministrationQuery {
    CaseAdministrationQuery::new(20, None, status, profile, title, nuc, None).unwrap()
}
#[test]
fn staff_permissions_membership_and_client_basic_projection_are_rechecked() {
    let Some(mut f) = Fixture::new() else { return };
    let litigator = f.user("litigator", true);
    let paralegal = f.user("paralegal", true);
    let client = f.user("client", true);
    let foreign = f.user("litigator", false);
    let store = f.store();
    for actor in [f.owner, litigator, paralegal] {
        store.get_administration(actor, f.case, f.at).unwrap();
    }
    for actor in [client, paralegal] {
        assert!(matches!(
            store.replace_administration(
                actor,
                f.case,
                CaseRevisionExpectation::new(0),
                CaseEditableValues::new(CaseMetadata::new("Edit", "REF").unwrap(), None),
                f.at
            ),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    assert!(matches!(
        store.get_administration(client, f.case, f.at),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(
        store.get_basic(client, f.case, f.at).unwrap().title,
        "Baseline"
    );
    assert!(matches!(
        store.get_administration(foreign, f.case, f.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(store
        .list_administrations(
            foreign,
            query(CaseStatusFilter::All, CaseProfileFilter::All, None, None),
            f.at
        )
        .unwrap()
        .cases
        .is_empty());
    store
        .remove_member(f.case, litigator, f.owner, f.at)
        .unwrap();
    assert!(matches!(
        store.get_administration(litigator, f.case, f.at),
        Err(ApplicationError::CaseNotFound)
    ));
    f.admin
        .execute(
            "UPDATE users SET active=FALSE WHERE id=$1",
            &[&paralegal.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.get_administration(paralegal, f.case, f.at),
        Err(ApplicationError::InvalidSession)
    ));
}
#[test]
fn filters_use_literal_current_heads_and_keyset_cursors_include_closed_identifiers() {
    let Some(mut f) = Fixture::new() else { return };
    let actor = f.user("litigator", false);
    let store = f.store();
    let mut ids = Vec::new();
    for n in 0..3 {
        let id = CaseId::new();
        store
            .register_penal(
                actor,
                id,
                PenalCaseCreation::new(
                    CaseMetadata::new(&format!("Literal_% {n}"), "REF").unwrap(),
                    profile(&format!("N{n}")),
                ),
                f.at,
            )
            .unwrap();
        ids.push(id);
    }
    let foreign = CaseId::new();
    store
        .register_penal(
            f.owner,
            foreign,
            PenalCaseCreation::new(
                CaseMetadata::new("Literal_% secret", "REF").unwrap(),
                profile("Secret"),
            ),
            f.at,
        )
        .unwrap();
    let page = store
        .list_administrations(
            actor,
            query(
                CaseStatusFilter::Active,
                CaseProfileFilter::Complete,
                Some("_%"),
                None,
            ),
            f.at,
        )
        .unwrap();
    assert_eq!(page.cases.len(), 3);
    assert!(store
        .list_administrations(
            actor,
            query(
                CaseStatusFilter::All,
                CaseProfileFilter::All,
                Some("literal"),
                None
            ),
            f.at
        )
        .unwrap()
        .cases
        .is_empty());
    ids.sort_by_key(|id| id.as_uuid());
    let mut cursor = None;
    for expected in &ids {
        let q = CaseAdministrationQuery::new(
            1,
            cursor,
            CaseStatusFilter::All,
            CaseProfileFilter::All,
            None,
            None,
            None,
        )
        .unwrap();
        let page = store.list_administrations(actor, q, f.at).unwrap();
        assert_eq!(page.cases[0].origin.id, *expected);
        cursor = page.next_after_id;
    }
    assert!(cursor.is_none());
    let first = store.get_administration(actor, ids[0], f.at).unwrap();
    let old = first
        .administration
        .values()
        .profile()
        .unwrap()
        .nuc()
        .to_owned();
    store
        .replace_administration(
            actor,
            ids[0],
            CaseRevisionExpectation::new(1),
            CaseEditableValues::new(
                CaseMetadata::new("Edited", "REF").unwrap(),
                Some(profile("Corrected")),
            ),
            f.at,
        )
        .unwrap();
    store
        .change_administrative_status(
            actor,
            ids[0],
            CaseRevisionExpectation::new(2),
            CaseAdministrativeStatus::Closed,
            f.at,
        )
        .unwrap();
    assert!(store
        .list_administrations(
            actor,
            query(
                CaseStatusFilter::All,
                CaseProfileFilter::All,
                None,
                Some(&old)
            ),
            f.at
        )
        .unwrap()
        .cases
        .is_empty());
    let closed = store
        .list_administrations(
            actor,
            query(
                CaseStatusFilter::Closed,
                CaseProfileFilter::Complete,
                None,
                Some("Corrected"),
            ),
            f.at,
        )
        .unwrap();
    assert_eq!(closed.cases.len(), 1);
    assert!(matches!(
        store.register_penal(
            actor,
            CaseId::new(),
            PenalCaseCreation::new(
                CaseMetadata::new("Collision", "REF").unwrap(),
                profile("Corrected")
            ),
            f.at
        ),
        Err(ApplicationError::CaseIdentifierConflict)
    ));
    let history = store
        .administration_history(
            actor,
            ids[0],
            CaseAdministrationHistoryQuery::new(2, None).unwrap(),
            f.at,
        )
        .unwrap();
    assert_eq!(
        history
            .revisions
            .iter()
            .map(|r| r.revision.get())
            .collect::<Vec<_>>(),
        vec![3, 2]
    );
    assert_eq!(history.next_before_revision.unwrap().get(), 2);
    let last = store
        .administration_history(
            actor,
            ids[0],
            CaseAdministrationHistoryQuery::new(2, Some(2)).unwrap(),
            f.at,
        )
        .unwrap();
    assert_eq!(last.revisions[0].revision.get(), 1);
    assert!(!last.has_more);
}
