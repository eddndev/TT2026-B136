use super::*;

#[tokio::test]
async fn foreign_or_mismatched_workflow_responses_are_opaque_internal_errors() {
    for fault in 0..6 {
        let mut saved = creation();
        match fault {
            0 => saved.hearing.review.case_id = domain::cases::CaseId::new(),
            1 => saved.origin.association_id = ResourceActivityId::new(),
            2 => saved.hearing.review.command.resource.id = ResourceId::new(),
            3 => saved.association.receipt.operation_id = ResourceActivityOperationId::new(),
            4 => saved.origin.submission_digest = domain::crypto::Sha256Digest::from_array([9; 32]),
            _ => {
                saved.association.sources.resource.recorded_by.email =
                    "different@example.test".into()
            }
        }
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(saved));
        let (status, value) = request(
            write,
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(submission()),
        )
        .await;
        assert_eq!((status, value), (500, internal()), "fault {fault}");
    }
    let mut write = MockWrite::new();
    write.expect_prepare().times(1).return_once(|_, _, _, _| {
        let mut d = draft();
        d.command.hearing_id = ResourceHearingId::new();
        Ok(d)
    });
    let (status, value) = request(
        write,
        MockRead::new(),
        "POST",
        &format!("{}/prepare", base()),
        Some(body()),
    )
    .await;
    assert_eq!((status, value), (500, internal()));
    let mut read = MockRead::new();
    read.expect_get()
        .times(1)
        .return_once(|_, _, _, _, _| Ok(creation()));
    let (status, value) = request(
        MockWrite::new(),
        read,
        "GET",
        &format!("{}/{}/revisions/2", base(), hearing_id()),
        None,
    )
    .await;
    assert_eq!((status, value), (500, internal()));
}

#[tokio::test]
async fn incidental_material_order_is_valid_but_reassigned_provenance_is_rejected() {
    for altered in [false, true] {
        let mut saved = creation_with_people();
        saved.hearing.material.participants.reverse();
        if altered {
            use application::typed_participants::ParticipantRevisionSnapshot;
            let ParticipantRevisionSnapshot::Manual(person) =
                &mut saved.hearing.material.participants[0].revision
            else {
                unreachable!()
            };
            person.changed_by.email = "another@example.test".into();
        }
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(saved));
        let (status, value) = request(
            MockWrite::new(),
            read,
            "GET",
            &format!("{}/{}", base(), hearing_id()),
            None,
        )
        .await;
        assert_eq!(status, if altered { 500 } else { 200 }, "{value}");
        if altered {
            assert_eq!(value, internal());
        }
    }
}
