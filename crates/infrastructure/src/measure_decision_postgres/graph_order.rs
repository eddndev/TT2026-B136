use super::{graph::Key, inconsistent};
use application::ApplicationError;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn order(mut edges: BTreeMap<Key, BTreeSet<Key>>) -> Result<Vec<Key>, ApplicationError> {
    if edges.values().flatten().any(|key| !edges.contains_key(key)) {
        return Err(inconsistent("durable graph dependency is absent"));
    }
    let mut ordered = Vec::with_capacity(edges.len());
    while !edges.is_empty() {
        let next = edges
            .iter()
            .find(|(_, parents)| parents.is_empty())
            .map(|(key, _)| *key)
            .ok_or_else(|| inconsistent("durable hearing and decision graph contains a cycle"))?;
        edges.remove(&next);
        for parents in edges.values_mut() {
            parents.remove(&next);
        }
        ordered.push(next);
    }
    Ok(ordered)
}
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    fn group(id: u128) -> Key {
        Key::Group(Uuid::from_u128(id))
    }
    fn hearing(id: u128, revision: u32) -> Key {
        Key::Hearing(Uuid::from_u128(id), revision)
    }
    #[test]
    fn older_anchor_precedes_a_later_review_of_its_own_decision() {
        let h1 = hearing(1, 1);
        let h2 = hearing(1, 2);
        let g = group(2);
        let ordered = order(BTreeMap::from([
            (h1, BTreeSet::new()),
            (g, BTreeSet::from([h1])),
            (h2, BTreeSet::from([g, h1])),
        ]))
        .unwrap();
        assert!(ordered.iter().position(|k| *k == h1) < ordered.iter().position(|k| *k == g));
        assert!(ordered.iter().position(|k| *k == g) < ordered.iter().position(|k| *k == h2));
    }
    #[test]
    fn empty_inventory_has_no_reconstruction_work() {
        assert!(order(BTreeMap::new()).unwrap().is_empty());
    }
    #[test]
    fn a_cycle_in_an_older_hearing_prefix_rejects_the_entire_plan() {
        let h1 = hearing(1, 1);
        let h2 = hearing(1, 2);
        let g = group(2);
        assert!(order(BTreeMap::from([
            (h1, BTreeSet::from([g])),
            (h2, BTreeSet::from([h1])),
            (g, BTreeSet::from([h2])),
            (group(3), BTreeSet::new())
        ]))
        .is_err());
    }
    #[test]
    fn a_missing_dependency_cannot_be_silently_omitted() {
        assert!(order(BTreeMap::from([(group(1), BTreeSet::from([group(2)]))])).is_err());
    }
}
