use application::{identity::owner_certificates::OwnerBindingCommit, ApplicationError};

use crate::{submission_support::count, support::*};

#[test]
fn withdrawal_rechecks_current_principal_after_applied_or_existing_commit_without_retry() {
    let mut outcomes = Vec::new();
    for existing in [false, true] {
        for revoked in [true, false] {
            let mut harness = Harness::new();
            let observed = harness.state.clone();
            {
                let mut state = observed.lock().unwrap();
                state.now = 3000;
                state.context.trust = None;
                state.context.account.revision = 10;
            }
            harness.withdrawal(receipt());
            let changed = observed.clone();
            harness
                .store
                .expect_commit_withdrawal()
                .times(1)
                .returning(move |_| {
                    let mut state = changed.lock().unwrap();
                    state.calls.push("commit_withdrawal");
                    // The store already has a terminal receipt when authority changes.
                    state.found = Some(retired());
                    if revoked {
                        state.principal = None;
                    } else {
                        state.principal.as_mut().unwrap().email =
                            "changed-owner@example.test".into();
                    }
                    Ok(if existing {
                        OwnerBindingCommit::Existing(retired())
                    } else {
                        OwnerBindingCommit::Applied(retired())
                    })
                });

            let result = harness.service().withdraw("session", binding(), 1);
            assert_eq!(count(&observed, "commit_withdrawal"), 1);
            assert_eq!(count(&observed, "load_withdrawal"), 1);
            assert_eq!(observed.lock().unwrap().found, Some(retired()));
            outcomes.push((
                existing,
                revoked,
                matches!(result, Err(ApplicationError::InvalidSession)),
            ));
        }
    }
    assert!(
        outcomes.iter().all(|(_, _, rejected)| *rejected),
        "withdrawal released evidence after authority changed (existing, revoked, rejected): {outcomes:?}"
    );
}
