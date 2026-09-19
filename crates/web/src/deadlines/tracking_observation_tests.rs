use super::{tracking_projection as projection, tracking_projection_test_support as fixture};
use application::deadline_reevaluation::{DependencyFamily, ObservationRole, Observations};
use domain::cases::CaseId;
use serde_json::json;
use uuid::Uuid;

#[test]
fn tracking_http_observations_preserve_exact_roles_scope_parents_and_supplied_digests() {
    let value = projection::observations(&fixture::full_observations(), fixture::case()).unwrap();
    assert_eq!(
        value,
        json!({"case_id": Uuid::nil(), "entries": [
            {
                "role": "profile", "family": "profile", "id": Uuid::nil(), "revision": 1,
                "case_id": null, "hearing_id": null, "parent_resolution": null,
                "submission_digest": "11".repeat(32), "evidence_digest": "22".repeat(32),
            },
            {
                "role": "source", "family": "notification", "id": Uuid::from_u128(2),
                "revision": 2, "case_id": Uuid::nil(), "hearing_id": null,
                "parent_resolution": {"id": Uuid::from_u128(3), "revision": 1},
                "submission_digest": "11".repeat(32), "evidence_digest": "22".repeat(32),
            },
            {
                "role": "calendar", "family": "calendar", "id": Uuid::from_u128(4),
                "revision": 3, "case_id": null, "hearing_id": null, "parent_resolution": null,
                "submission_digest": "11".repeat(32), "evidence_digest": "22".repeat(32),
            },
            {
                "role": "notification_parent", "family": "resolution", "id": Uuid::from_u128(3),
                "revision": 4, "case_id": Uuid::nil(), "hearing_id": null, "parent_resolution": null,
                "submission_digest": "11".repeat(32), "evidence_digest": "22".repeat(32),
            },
        ]})
    );
}

#[test]
fn tracking_http_observations_project_private_profiles_and_hearing_scope() {
    let mut observations = fixture::full_observations();
    observations.entries.truncate(2);
    observations.entries[0].case_id = Some(fixture::case());
    let source = &mut observations.entries[1];
    source.family = DependencyFamily::HearingResult;
    source.hearing_id = Some(Uuid::nil());
    source.parent_resolution = None;
    let value = projection::observations(&observations, fixture::case()).unwrap();
    assert_eq!(value["entries"][0]["case_id"], json!(Uuid::nil()));
    assert_eq!(value["entries"][1]["family"], "hearing_result");
    assert_eq!(value["entries"][1]["hearing_id"], json!(Uuid::nil()));
    assert!(value["entries"][1]["parent_resolution"].is_null());
    observations.entries[1].family = DependencyFamily::Resolution;
    observations.entries[1].hearing_id = None;
    let value = projection::observations(&observations, fixture::case()).unwrap();
    assert_eq!(value["entries"][1]["family"], "resolution");
}

#[test]
fn tracking_http_observations_reject_malformed_structure_without_repairing_it() {
    let mut invalid = Vec::<(&str, Observations)>::new();
    let base = fixture::full_observations();
    let mut value = base.clone();
    value.entries.clear();
    invalid.push(("empty", value));
    let mut value = base.clone();
    value.entries.remove(0);
    invalid.push(("no profile", value));
    let mut value = base.clone();
    value.entries.swap(1, 2);
    invalid.push(("unordered", value));
    let mut value = base.clone();
    value.entries[1] = value.entries[0].clone();
    invalid.push(("duplicate role", value));
    let mut value = base.clone();
    value.entries[0].revision = 0;
    invalid.push(("zero revision", value));
    let mut value = base.clone();
    value.entries[0].family = DependencyFamily::Calendar;
    invalid.push(("role family mismatch", value));
    let mut value = base.clone();
    value.entries[0].parent_resolution = value.entries[1].parent_resolution;
    invalid.push(("parent on profile", value));
    let mut value = base.clone();
    value.entries[1].parent_resolution = None;
    invalid.push(("notification missing parent", value));
    let mut value = base.clone();
    value.entries[1].case_id = Some(CaseId::from_uuid(Uuid::from_u128(99)));
    invalid.push(("foreign source", value));
    let mut value = base.clone();
    value.entries[1].family = DependencyFamily::HearingResult;
    value.entries[1].parent_resolution = None;
    value.entries.pop();
    invalid.push(("hearing without root", value));
    let mut value = base.clone();
    value.entries[2].case_id = Some(fixture::case());
    invalid.push(("scoped calendar", value));
    let mut value = base.clone();
    value.entries[3].id = Uuid::from_u128(99);
    invalid.push(("foreign parent root", value));
    let mut value = base.clone();
    value.entries[3].revision = 1;
    value.entries[1]
        .parent_resolution
        .as_mut()
        .unwrap()
        .revision = 2;
    invalid.push(("parent older than linked revision", value));
    let mut value = base.clone();
    value.entries.remove(1);
    invalid.push(("observed parent without notification", value));
    let mut value = base.clone();
    value.entries.push(value.entries[3].clone());
    invalid.push(("too many observations", value));
    let mut value = base.clone();
    value.entries[0].case_id = Some(CaseId::from_uuid(Uuid::from_u128(99)));
    invalid.push(("foreign profile", value));
    let mut value = base.clone();
    value.entries[1].role = ObservationRole::Calendar;
    invalid.push(("notification in calendar role", value));
    for (name, value) in invalid {
        assert!(
            projection::observations(&value, fixture::case()).is_err(),
            "{name}"
        );
    }
    assert!(projection::observations(&base, CaseId::from_uuid(Uuid::from_u128(99))).is_err());
}
