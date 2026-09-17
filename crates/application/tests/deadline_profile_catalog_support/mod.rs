#![allow(dead_code)]
pub use crate::case_support::{instant, CountingClock, MockIdentity};
use application::{deadline_profiles::*, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    DomainError,
};
use mockall::mock;
use std::{io::Read, sync::Arc};
mod values;
pub use values::*;

mock! {
    pub Store {}
    impl DeadlineProfileStore for Store {
        fn list(&self,actor:UserId,collection:DeadlineProfileCollection,query:DeadlineProfileQuery,at:OffsetDateTime)->Result<DeadlineProfilePage,ApplicationError>;
        fn get(&self,actor:UserId,collection:DeadlineProfileCollection,id:DeadlineProfileId,revision:Option<DeadlineProfileRevision>,at:OffsetDateTime)->Result<DeadlineProfileDetail,ApplicationError>;
        fn history(&self,actor:UserId,collection:DeadlineProfileCollection,id:DeadlineProfileId,query:DeadlineProfileHistoryQuery,at:OffsetDateTime)->Result<DeadlineProfileHistoryPage,ApplicationError>;
        fn prepare(&self,actor:UserId,collection:DeadlineProfileCollection,command:&DeadlineProfileCommand)->Result<DeadlineProfilePreparation,ApplicationError>;
        fn commit(&self,actor:UserId,prepared:PreparedDeadlineProfileChange)->Result<DeadlineProfileDetail,ApplicationError>;
    }
}

pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut bytes = [0u8; 32];
        for (i, b) in data.iter().enumerate() {
            let slot = i % 32;
            bytes[slot] = bytes[slot].wrapping_add(*b).wrapping_add(i as u8);
        }
        Sha256Digest::from_array(bytes)
    }
    fn hash_stream(&self, r: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = vec![];
        r.read_to_end(&mut bytes)
            .map_err(|e| DomainError::StreamRead {
                message: e.to_string(),
            })?;
        Ok(self.hash_bytes(&bytes))
    }
}
pub fn command() -> DeadlineProfileCommand {
    DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: DeadlineProfileId::new(),
        change: DeadlineProfileChange::Publish {
            definition: definition(None),
        },
    }
}
pub fn empty(
    collection: DeadlineProfileCollection,
    command: &DeadlineProfileCommand,
) -> DeadlineProfilePreparation {
    DeadlineProfilePreparation {
        collection,
        profile_id: command.profile_id,
        base: None,
        initial_scope: None,
    }
}
pub fn preparation(
    collection: DeadlineProfileCollection,
    base: &DeadlineProfileDetail,
) -> DeadlineProfilePreparation {
    DeadlineProfilePreparation {
        collection,
        profile_id: base.id,
        base: Some(base.clone()),
        initial_scope: Some(base.definition.scope().clone()),
    }
}
pub fn replacement(base: &DeadlineProfileDetail) -> DeadlineProfileCommand {
    DeadlineProfileCommand {
        profile_id: base.id,
        operation_id: DeadlineProfileOperationId::new(),
        change: DeadlineProfileChange::Replace {
            expected_revision: base.revision,
            definition: base.definition.clone(),
            reason: text("Rechecked declared rule"),
        },
    }
}
pub fn retirement(base: &DeadlineProfileDetail) -> DeadlineProfileCommand {
    DeadlineProfileCommand {
        profile_id: base.id,
        operation_id: DeadlineProfileOperationId::new(),
        change: DeadlineProfileChange::Retire {
            expected_revision: base.revision,
            reason: text("Retired configuration"),
        },
    }
}
pub fn detail(
    actor: UserId,
    command: &DeadlineProfileCommand,
    definition: DeadlineProfileDefinition,
) -> DeadlineProfileDetail {
    let definition_digest = deadline_profile_definition_digest(&Hasher, &definition);
    let algorithm = DeadlineProfileAlgorithm::V1;
    DeadlineProfileDetail {
        id: command.profile_id,
        revision: command.result_revision().unwrap(),
        definition,
        definition_digest,
        algorithm,
        status: command.result_status(),
        reason: command.reason().cloned(),
        receipt: DeadlineProfileReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            submission_digest: deadline_profile_submission_digest(
                &Hasher,
                actor,
                command,
                algorithm,
                definition_digest,
            ),
        },
        recorded_at: instant(),
        recorded_by: DeadlineProfileActorSnapshot {
            id: actor,
            email: "owner@example.com".into(),
        },
    }
}
pub fn service(
    store: MockStore,
    identity: MockIdentity,
) -> (DeadlineProfileService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        DeadlineProfileService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(Hasher),
            clock.clone(),
        ),
        clock,
    )
}
pub fn query(limit: u32) -> DeadlineProfileQuery {
    DeadlineProfileQuery::new(limit, None, DeadlineProfileStatusFilter::All).unwrap()
}
