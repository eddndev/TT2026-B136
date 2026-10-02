use super::{support::*, Fixture};
use application::{
    alerts::AlertSchedulerStore, deadline_dispatch::*, deadline_worker::DeadlineWorkerStore,
    ApplicationError, PortFailureKind,
};
use infrastructure::{
    with_validated_postgres, PostgresAlertStore, PostgresDeadlineDispatchStore,
    PostgresDeadlineWorkerStore, RingSha256Hasher, SystemClock,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn reconnecting_consumers_validate_the_full_inventory_without_retaining_the_capability() {
    let Some(mut db) = Fixture::new() else { return };
    let (url, application) = bounded_url(&db);
    let (alerts, dispatch, worker) = with_validated_postgres(&url, |source| {
        let hasher = Arc::new(RingSha256Hasher);
        let clock = Arc::new(SystemClock::new());
        Ok::<_, ApplicationError>((
            PostgresAlertStore::open(source, hasher.clone(), clock.clone(), None)?,
            PostgresDeadlineDispatchStore::open(source, hasher.clone(), clock.clone())?,
            PostgresDeadlineWorkerStore::open(source, hasher, clock)?,
        ))
    })
    .unwrap();
    corrupt_case_actor_email(&mut db);
    let before = db.snapshot();
    let pids = backend_pids(&mut db.admin, &application);
    assert_eq!(pids.len(), 3, "only the owned consumers survive the scope");
    for pid in pids {
        let terminated: bool = db
            .admin
            .query_one("SELECT pg_terminate_backend($1, 5000::bigint)", &[&pid])
            .unwrap()
            .get(0);
        assert!(
            terminated,
            "test backend must terminate before reconnecting"
        );
    }
    require_inventory_rejection(|| alerts.run_next());
    let request = || DeadlineDispatchRequest {
        stream: DeadlineDispatchStream::Events,
        limit: DeadlineDispatchLimit::new(1).unwrap(),
    };
    require_inventory_rejection(|| dispatch.dispatch(request()));
    require_inventory_rejection(|| worker.run_next());
    assert_eq!(db.snapshot(), before);
}

fn require_inventory_rejection<T>(mut operation: impl FnMut() -> Result<T, ApplicationError>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let error = match operation() {
            Ok(_) => panic!("a consumer succeeded after persisted inventory corruption"),
            Err(error) => error,
        };
        if matches!(error, ApplicationError::InvalidConfiguration(_)) {
            assert_inventory_error(error);
            return;
        }
        // Server termination and the synchronous client's channel-close signal
        // are separate events. Only the expected closing-transport errors may
        // precede the precise inventory rejection, never a successful operation.
        let closing = match &error {
            ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Unavailable,
                ..
            } => true,
            ApplicationError::Port(message) => matches!(
                message.as_str(),
                "audit database: error communicating with the server"
                    | "audit database: connection closed"
            ),
            _ => false,
        };
        assert!(
            closing,
            "unexpected failure while closing the old socket: {error}"
        );
        assert!(
            Instant::now() < deadline,
            "socket-close signal did not settle: {error}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
