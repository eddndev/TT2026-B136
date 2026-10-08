use super::*;
use std::sync::{Arc, Mutex};

#[test]
fn clients_are_denied_before_any_read_lookup() {
    let saved = operation(40);
    let mut reader = saved.capture.review.actor.clone();
    reader.role = Role::Client;
    for kind in READS {
        let service = service(MockReads::new(), identity(&reader, 1), clock());
        assert!(matches!(
            read(&service, &saved, kind),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn invalid_sessions_never_reach_a_read_store() {
    let saved = operation(40);
    for kind in READS {
        let mut identity = crate::case_support::MockIdentity::new();
        identity
            .expect_authenticate()
            .times(1)
            .return_once(|_| Err(ApplicationError::InvalidSession));
        assert!(matches!(
            read(&service(MockReads::new(), identity, clock()), &saved, kind),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn each_read_reauthenticates_the_complete_current_principal_before_disclosure() {
    let saved = operation(40);
    let actor = saved.capture.review.actor.clone();
    for kind in READS {
        for mutation in 0..4 {
            let mut changed = actor.clone();
            match mutation {
                0 => changed.id = UserId::from_uuid(uuid::Uuid::from_u128(999)),
                1 => changed.email = "changed-reader@example.test".into(),
                2 => changed.role = Role::Paralegal,
                _ => {}
            }
            let mut identity = crate::case_support::MockIdentity::new();
            let mut sequence = mockall::Sequence::new();
            let before = actor.clone();
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(before));
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| {
                    if mutation == 3 {
                        Err(ApplicationError::InvalidSession)
                    } else {
                        Ok(changed)
                    }
                });
            let store = successful_store(&actor, &saved, saved.clone(), kind);
            assert!(matches!(
                read(&service(store, identity, clock()), &saved, kind),
                Err(ApplicationError::InvalidSession)
            ));
        }
    }
}

#[test]
fn authorization_absence_and_audit_failures_are_preserved() {
    let saved = operation(40);
    let actor = &saved.capture.review.actor;
    for kind in READS {
        for failure in 0..4 {
            let error = match failure {
                0 => ApplicationError::CaseNotFound,
                1 => PrecautionaryHearingError::NotFound.into(),
                2 => ApplicationError::PermissionDenied,
                _ => ApplicationError::Port("read audit failed".into()),
            };
            let expected = error.to_string();
            let mut store = MockReads::new();
            match kind {
                ReadKind::List => {
                    store
                        .expect_list()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
                ReadKind::Get => {
                    store
                        .expect_get()
                        .times(1)
                        .return_once(move |_, _, _, _| Err(error));
                }
                ReadKind::Operation => {
                    store
                        .expect_get_operation()
                        .times(1)
                        .return_once(move |_, _, _| Err(error));
                }
            }
            assert_eq!(
                read(&service(store, identity(actor, 1), clock()), &saved, kind)
                    .unwrap_err()
                    .to_string(),
                expected,
            );
        }
    }
}

#[test]
fn every_read_rejects_future_capture_and_invalid_or_regressing_service_clocks() {
    let saved = operation(40);
    let actor = &saved.capture.review.actor;
    let utc = at() + time::Duration::seconds(10);
    let non_utc = utc.to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    let year_zero = time::Date::from_calendar_date(0, time::Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    for kind in READS {
        for times in [
            vec![at() - time::Duration::seconds(1); 2],
            vec![utc, non_utc],
            vec![utc, year_zero],
            vec![utc, utc - time::Duration::seconds(1)],
        ] {
            let store = successful_store(actor, &saved, saved.clone(), kind);
            let clock = Arc::new(ReadClock(Mutex::new(times)));
            let mut identity = crate::case_support::MockIdentity::new();
            let actor = actor.clone();
            identity
                .expect_authenticate()
                .times(1..=2)
                .returning(move |_| Ok(actor.clone()));
            assert!(read(&service(store, identity, clock), &saved, kind).is_err());
        }
        for started in [non_utc, year_zero] {
            let clock = Arc::new(ReadClock(Mutex::new(vec![started])));
            assert!(read(
                &service(MockReads::new(), identity(actor, 1), clock),
                &saved,
                kind,
            )
            .is_err());
        }
    }
}
