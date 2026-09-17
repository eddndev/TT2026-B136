use crate::deadline_input_support::*;
use application::deadline_inputs::*;

#[test]
fn hearing_result_keeps_the_selected_zero_uuid_agreement_after_head_removal_and_withdrawal() {
    use application::procedural_facts::FactHearingRef;
    use domain::deadline_triggers::TriggerSourceRef;
    for instant in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let first = hearing_result(&mut db, instant);
        let corrected = corrected_result(&db, &first);
        let head = retired_result(&db, &corrected);
        assert!(head.snapshot.values.agreements().is_empty());
        let request = request(
            db.case,
            TriggerSourceRef::HearingResult(FactHearingRef {
                hearing_id: first.snapshot.hearing_id,
                result_id: first.snapshot.id,
                revision: first.snapshot.revision,
                agreement_id: Some(
                    domain::hearing_results::HearingResultAgreementId::from_uuid(uuid::Uuid::nil()),
                ),
            }),
        );
        let reader = store(&db);
        let before = audit(&mut db);
        let material = reader.load(db.owner, &request).unwrap();
        assert_eq!(
            material.source,
            Some(DeadlineSourceDetail::HearingResult(Box::new(first.clone())))
        );
        assert_eq!(
            material.source_head,
            Some(DeadlineSourceDetail::HearingResult(Box::new(head)))
        );
        let calculation =
            check_deadline_inputs(&infrastructure::RingSha256Hasher, &request, &material).unwrap();
        let domain::deadline_triggers::TriggerOutcome::Extracted { at } =
            calculation.trigger().outcome()
        else {
            panic!("source time expected")
        };
        assert_eq!(
            at.local_date().map(|date| date.date()),
            Some(first.snapshot.values.event_time().local_date())
        );
        assert_eq!(
            at.offset(),
            Some(first.snapshot.values.event_time().offset())
        );
        if instant {
            assert_eq!(
                (at.local_hour(), at.local_minute(), at.local_second()),
                (Some(14), Some(15), Some(16))
            );
        } else {
            assert_eq!(
                (at.local_hour(), at.local_minute(), at.local_second()),
                (None, None, None)
            );
        }
        assert_one_read(&mut db, &before);
    }
}

#[test]
fn a_removed_agreement_cannot_be_selected_from_the_newer_exact_revision() {
    use application::{procedural_facts::FactHearingRef, ApplicationError};
    use domain::deadline_triggers::TriggerSourceRef;
    let Some(mut db) = Fixture::new() else { return };
    let first = hearing_result(&mut db, true);
    let head = corrected_result(&db, &first);
    let reader = store(&db);
    let request = request(
        db.case,
        TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: head.snapshot.hearing_id,
            result_id: head.snapshot.id,
            revision: head.snapshot.revision,
            agreement_id: Some(
                domain::hearing_results::HearingResultAgreementId::from_uuid(uuid::Uuid::nil()),
            ),
        }),
    );
    let before = audit(&mut db);
    assert!(matches!(
        reader.load(db.owner, &request),
        Err(ApplicationError::DeadlineInput(
            DeadlineInputError::Inconsistent(_)
        ))
    ));
    assert_eq!(audit(&mut db), before);
}
