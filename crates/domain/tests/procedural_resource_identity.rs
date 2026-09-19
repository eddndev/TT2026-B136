use domain::{cases::CaseId, procedural_resources::*, DomainError};
use uuid::Uuid;

#[test]
fn independent_roots_and_positive_revisions_do_not_bind_resources_to_case_stages() {
    let id = Uuid::from_u128(8);
    let resource = ResourceId::from_uuid(id);
    let case = CaseId::from_uuid(Uuid::from_u128(9));
    let root = ResourceRoot::new(resource, case);
    assert_eq!((root.id(), root.case_id()), (resource, case));
    let act_id = ResourceActId::from_uuid(Uuid::from_u128(11));
    let act = ResourceActRoot::new(act_id, resource);
    assert_eq!((act.id(), act.resource_id()), (act_id, resource));
    assert_eq!(resource.to_string(), id.to_string());
    assert_eq!(ResourceOperationId::from_uuid(id).as_uuid(), id);
    assert_eq!(
        ResourceOperationId::from_uuid(id).to_string(),
        id.to_string()
    );
    assert_eq!(act_id.to_string(), act_id.as_uuid().to_string());
    assert_ne!(ResourceId::new(), ResourceId::default());
    assert_ne!(ResourceActId::new(), ResourceActId::default());
    assert_ne!(ResourceOperationId::new(), ResourceOperationId::default());
    assert_eq!(ResourceRevision::initial().next().unwrap().get(), 2);
    assert_eq!(ResourceActRevision::initial().next().unwrap().get(), 2);
    assert_eq!(ResourceRevision::new(u32::MAX).unwrap().next(), None);
    assert_eq!(ResourceActRevision::new(u32::MAX).unwrap().next(), None);
    assert_eq!(
        ResourceRevision::try_from(0),
        Err(DomainError::InvalidProceduralResource("revision"))
    );
    assert_eq!(
        ResourceActRevision::try_from(0),
        Err(DomainError::InvalidProceduralResource("act_revision"))
    );
}

#[test]
fn explicit_catalogs_reject_unknown_tags_and_keep_archive_separate_from_withdrawal() {
    for (value, tag, wire) in [
        (ResourceKind::Revocation, 0, "revocation"),
        (ResourceKind::Appeal, 1, "appeal"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, wire));
        assert_eq!(wire.parse::<ResourceKind>().unwrap(), value);
    }
    for (value, tag, wire) in [
        (ResourceMode::Oral, 0, "oral"),
        (ResourceMode::Written, 1, "written"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, wire));
        assert_eq!(wire.parse::<ResourceMode>().unwrap(), value);
    }
    for (value, tag, wire) in [
        (ResourceStatus::Active, 0, "active"),
        (ResourceStatus::Archived, 1, "archived"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, wire));
        assert_eq!(wire.parse::<ResourceStatus>().unwrap(), value);
        assert!(wire.parse::<ResourceActKind>().is_err());
    }
    for (value, tag, wire) in [
        (ResourceActKind::Interposition, 0, "interposition"),
        (ResourceActKind::Admission, 1, "admission"),
        (ResourceActKind::Inadmissibility, 2, "inadmissibility"),
        (ResourceActKind::Withdrawal, 3, "withdrawal"),
        (ResourceActKind::Resolution, 4, "resolution"),
    ] {
        assert_eq!((value.tag(), value.as_str()), (tag, wire));
        assert_eq!(wire.parse::<ResourceActKind>().unwrap(), value);
        assert!(wire.parse::<ResourceStatus>().is_err());
    }
    for invalid in ["", "Appeal", " oral", "written ", "unknown", "2"] {
        assert!(invalid.parse::<ResourceKind>().is_err());
        assert!(invalid.parse::<ResourceMode>().is_err());
        assert!(invalid.parse::<ResourceStatus>().is_err());
        assert!(invalid.parse::<ResourceActKind>().is_err());
    }
}
