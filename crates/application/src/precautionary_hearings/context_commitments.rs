use super::{capture_validation::invalid, PrecautionaryContext};
use crate::{case_stages::CaseStageEntry, hearings::HearingDetail, ApplicationError};
use domain::{case_stages::CaseStage, crypto::Sha256Digest};
use std::collections::{btree_map::Entry, BTreeMap};
type Key = ([u8; 16], u32);

/// Partial historical references must agree with every supplied exact context.
#[derive(Default)]
pub(super) struct ContextCommitments {
    administrations: BTreeMap<Key, Sha256Digest>,
    stages: BTreeMap<Key, (CaseStage, Option<Sha256Digest>)>,
}

impl ContextCommitments {
    pub fn context(&mut self, context: &PrecautionaryContext) -> Result<(), ApplicationError> {
        let m = context.material();
        for a in [&m.administration, &m.stage_administration] {
            retain(
                &mut self.administrations,
                (*a.case_id.as_uuid().as_bytes(), a.revision.get()),
                a.values_digest,
            )?;
        }
        let s = &m.stage;
        let digest = match s {
            CaseStageEntry::Initial(_) => None,
            CaseStageEntry::Changed(s) => Some(s.values_digest),
        };
        retain(
            &mut self.stages,
            (*s.case_id().as_uuid().as_bytes(), s.stage_revision().get()),
            (s.stage(), digest),
        )
    }

    pub fn ordinary(&mut self, detail: &HearingDetail) -> Result<(), ApplicationError> {
        let s = &detail.snapshot;
        let case = *s.case_id.as_uuid().as_bytes();
        retain(
            &mut self.administrations,
            (case, s.recorded_administration_revision.get()),
            s.recorded_administration_digest,
        )?;
        let c = &s.scheduling_context;
        retain(
            &mut self.administrations,
            (case, c.administration_revision.get()),
            c.administration_digest,
        )?;
        retain(
            &mut self.stages,
            (case, c.stage_revision.get()),
            (c.stage, c.stage_digest),
        )
    }
}

fn retain<K: Ord, V: PartialEq>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
) -> Result<(), ApplicationError> {
    match map.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value);
        }
        Entry::Occupied(entry) if *entry.get() != value => {
            return Err(invalid(
                "historical context reference contradicts supplied evidence",
            ));
        }
        Entry::Occupied(_) => {}
    }
    Ok(())
}
