use super::*;
use domain::{procedural_resources::ResourceId, resource_hearings::ResourceHearingId};

#[test]
fn existing_subject_tags_keep_their_exact_three_element_bytes() {
    let case = Uuid::from_u128(1);
    let id = Uuid::from_u128(2);
    for (tag, subject) in [
        (
            0,
            AlertSubject::Hearing {
                case_id: CaseId::from_uuid(case),
                id: HearingId::from_uuid(id),
            },
        ),
        (
            1,
            AlertSubject::Deadline {
                case_id: CaseId::from_uuid(case),
                id: DeadlineId::from_uuid(id),
            },
        ),
    ] {
        let expected = format!("[{tag},\"{case}\",\"{id}\"]");
        assert_eq!(
            serde_json::to_vec(&super::subject(subject)).unwrap(),
            expected.as_bytes()
        );
        assert_eq!(
            read_subject(&serde_json::from_str::<Value>(&expected).unwrap()).unwrap(),
            subject
        );
    }
}

#[test]
fn own_subject_appends_exact_resource_parent_without_reusing_ordinary_tag() {
    let case = Uuid::from_u128(1);
    let id = Uuid::from_u128(2);
    let resource = Uuid::from_u128(3);
    let value = AlertSubject::ResourceHearing {
        case_id: CaseId::from_uuid(case),
        resource_id: ResourceId::from_uuid(resource),
        id: ResourceHearingId::from_uuid(id),
    };
    let expected = json!([2, case, id, resource]);
    assert_eq!(super::subject(value), expected);
    assert_eq!(subject_key(value), (2, id));
    assert_eq!(read_subject(&expected).unwrap(), value);
}

#[test]
fn own_subject_rejects_absent_null_noncanonical_and_additional_parent_material() {
    let case = Uuid::from_u128(1);
    let id = Uuid::from_u128(2);
    let resource = Uuid::from_u128(10);
    for value in [
        json!([2, case, id]),
        json!([2, case, id, null]),
        json!([2, case, id, resource.to_string().to_uppercase()]),
        json!([2, case, id, resource, "unexpected"]),
        json!({"0":2,"1":case,"2":id,"3":resource}),
    ] {
        assert!(
            read_subject(&value).is_err(),
            "accepted malformed own subject {value}"
        );
    }
}
