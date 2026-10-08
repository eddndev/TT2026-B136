mod authorization;
mod fixture;
mod history;
mod integrity;
mod lifecycle;
mod pagination;
pub use crate::measure_fixture::{FixedClock, Fixture, Seed, TestIdentity};
pub use application::{identity::Principal, measure_corrections::*, precautionary_measures::*};
pub use domain::{
    crypto::Sha256Digest,
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
};
pub use fixture::*;
pub use infrastructure::{PostgresMeasureDecisionStore, RingSha256Hasher};
pub use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresMeasureDecisionStore> {
    crate::measure_fixture::store(db)
}
pub fn reads(db: &Fixture, actor: Principal) -> MeasureRecordReadService {
    existing_reads(db, actor, store(db))
}
pub fn existing_reads(
    db: &Fixture,
    actor: Principal,
    storage: Arc<PostgresMeasureDecisionStore>,
) -> MeasureRecordReadService {
    MeasureRecordReadService::new(
        storage,
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    crate::administrative_fixture::snapshot(db)
}
