use super::*;
use std::sync::{Arc, Mutex};

#[test]
fn both_reads_reject_future_capture_non_utc_clock_and_clock_regression() {
    let f = Fixture::new(Role::Owner);
    let at = case_support::instant();
    for list in [false, true] {
        for fault in 0..4 {
            let capture_at = if fault == 0 {
                at + time::Duration::seconds(1)
            } else {
                at
            };
            let saved = creation(&f, f.command.hearing_id, capture_at);
            let offset = at.to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
            let times = match fault {
                0 => vec![at, at],
                1 => vec![offset, at],
                2 => vec![at, offset],
                _ => vec![at + time::Duration::seconds(1), at],
            };
            let store = if fault == 1 {
                let mut store = MockReads::new();
                if list {
                    let returned = page(&f, vec![saved]);
                    store
                        .expect_list()
                        .times(0..=1)
                        .return_once(move |_, _, _, _| Ok(returned));
                } else {
                    store
                        .expect_get()
                        .times(0..=1)
                        .return_once(move |_, _, _, _, _| Ok(saved));
                }
                store
            } else {
                successful_store(&f, &f.actor, saved, list)
            };
            let clock = Arc::new(ReadClock(Mutex::new(times)));
            stored_error(read(
                &service(store, identity(&f.actor, 1), clock),
                &f,
                list,
            ));
        }
    }
}

#[test]
fn both_reads_reauthenticate_the_full_principal_and_propagate_revocation() {
    let f = Fixture::new(Role::Owner);
    let saved = creation(&f, f.command.hearing_id, case_support::instant());
    for list in [false, true] {
        for fault in 0..4 {
            let mut changed = f.actor.clone();
            match fault {
                0 => changed.id = UserId::new(),
                1 => changed.email = "renamed@example.com".into(),
                2 => changed.role = Role::Paralegal,
                _ => {}
            }
            let after = if fault == 3 {
                Err(ApplicationError::InvalidSession)
            } else {
                Ok(changed)
            };
            let identity = changed_identity(f.actor.clone(), after);
            let store = successful_store(&f, &f.actor, saved.clone(), list);
            assert!(matches!(
                read(&service(store, identity, clock()), &f, list),
                Err(ApplicationError::InvalidSession)
            ));
        }
    }
}
