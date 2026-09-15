mod case_administration_support;
use application::cases::*;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::cases::CaseMetadata;

#[test]
fn basic_and_administration_reads_never_return_data_when_their_audit_insert_fails() {
    for action in [
        "case.read",
        "case.listed",
        "case.administration_read",
        "case.administration_listed",
        "case.administration_history_read",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let before = f.snapshot();
        f.admin
            .batch_execute(&format!(
                "ALTER TABLE audit_events ADD CONSTRAINT reject_read CHECK(action<>'{action}')"
            ))
            .unwrap();
        let result = match action {
            "case.read" => store.get_basic(f.owner, f.case, f.at).map(|_| ()),
            "case.listed" => store.list_basic(f.owner, 10, 0, f.at).map(|_| ()),
            "case.administration_read" => {
                store.get_administration(f.owner, f.case, f.at).map(|_| ())
            }
            "case.administration_listed" => store
                .list_administrations(
                    f.owner,
                    CaseAdministrationQuery::new(
                        10,
                        None,
                        CaseStatusFilter::All,
                        CaseProfileFilter::All,
                        None,
                        None,
                        None,
                    )
                    .unwrap(),
                    f.at,
                )
                .map(|_| ()),
            _ => store
                .administration_history(
                    f.owner,
                    f.case,
                    CaseAdministrationHistoryQuery::new(10, None).unwrap(),
                    f.at,
                )
                .map(|_| ()),
        };
        assert!(matches!(result, Err(ApplicationError::Port(_))), "{action}");
        assert_eq!(f.snapshot(), before);
    }
}
#[test]
fn failed_mutation_audit_preserves_the_prior_head_and_its_entire_history() {
    for action in [
        "case.administration_replaced",
        "case.administrative_status_changed",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let before = f.snapshot();
        f.admin
            .batch_execute(&format!(
                "ALTER TABLE audit_events ADD CONSTRAINT reject_mutation CHECK(action<>'{action}')"
            ))
            .unwrap();
        let result = if action == "case.administration_replaced" {
            store.replace_administration(
                f.owner,
                f.case,
                CaseRevisionExpectation::new(0),
                CaseEditableValues::new(CaseMetadata::new("Updated", "REF").unwrap(), None),
                f.at,
            )
        } else {
            store.change_administrative_status(
                f.owner,
                f.case,
                CaseRevisionExpectation::new(0),
                CaseAdministrativeStatus::Closed,
                f.at,
            )
        };
        assert!(matches!(result, Err(ApplicationError::Port(_))));
        assert_eq!(f.snapshot(), before);
    }
}
