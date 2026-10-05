pub(super) use super::record_bounds::bounds;
use super::{judicial_view::JudicialView, record_view::RecordView, wire::invalid, *};
use crate::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureEffect,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) struct HistoryView<'a> {
    pub judicial: &'a MeasureHistoryEvidence,
    pub administrative: &'a [MeasureAdministrativeEvidence],
    pub decisions: &'a [MeasureGroupEvidenceV2],
}
impl<'a> From<&'a MeasureRecordHistoryEvidence> for HistoryView<'a> {
    fn from(e: &'a MeasureRecordHistoryEvidence) -> Self {
        Self {
            judicial: &e.judicial,
            administrative: &e.administrative,
            decisions: &[],
        }
    }
}
impl<'a> From<&'a MeasureDecisionRecordHistoryEvidence> for HistoryView<'a> {
    fn from(e: &'a MeasureDecisionRecordHistoryEvidence) -> Self {
        Self {
            judicial: &e.records.judicial,
            administrative: &e.records.administrative,
            decisions: &e.decisions,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Member {
    Judicial(usize, usize),
    Decision(usize, usize),
    Administrative(usize),
}
type MemberKey = ([u8; 16], u32);
pub(super) struct RecordIndex<'a> {
    pub evidence: HistoryView<'a>,
    operations: BTreeSet<[u8; 16]>,
    decisions: BTreeSet<[u8; 16]>,
    members: BTreeMap<MemberKey, Member>,
}
impl<'a> RecordIndex<'a> {
    pub fn new(
        case: CaseId,
        evidence: HistoryView<'a>,
        reserve: usize,
    ) -> Result<Self, ApplicationError> {
        bounds(evidence, reserve)?;
        let mut result = Self {
            evidence,
            operations: BTreeSet::new(),
            decisions: BTreeSet::new(),
            members: BTreeMap::new(),
        };
        for (position, g) in evidence.judicial.groups.iter().enumerate() {
            if g.capture.review.case_id != case
                || g.origin != crate::precautionary_measures::legacy_group_origin(&g.capture)
            {
                return Err(invalid("judicial case or origin differs"));
            }
            result.identities(&g.capture.review.command)?;
            for (row, m) in g.capture.measures.iter().enumerate() {
                result.member(
                    (*m.result.id.as_uuid().as_bytes(), m.result.revision.get()),
                    Member::Judicial(position, row),
                )?;
            }
        }
        for (position, g) in evidence.decisions.iter().enumerate() {
            if g.capture.review.case_id != case
                || g.origin != super::decision_history::origin(&g.capture)
            {
                return Err(invalid("decision case or origin differs"));
            }
            result.identities(&g.capture.review.command)?;
            for (row, m) in g.capture.measures.iter().enumerate() {
                result.member(
                    (*m.result.id.as_uuid().as_bytes(), m.result.revision.get()),
                    Member::Decision(position, row),
                )?;
            }
        }
        for (position, a) in evidence.administrative.iter().enumerate() {
            if a.capture.review.case_id != case || a.origin != super::capture::origin(&a.capture) {
                return Err(invalid("administrative case or origin differs"));
            }
            result.operation(*a.capture.review.command.operation_id.as_uuid().as_bytes())?;
            let c = &a.capture.records[0];
            result.member(
                (*c.result.id.as_uuid().as_bytes(), c.result.revision.get()),
                Member::Administrative(position),
            )?;
        }
        Ok(result)
    }
    fn identities(&mut self, c: &MeasureDecisionCommand) -> Result<(), ApplicationError> {
        self.operation(*c.operation_id.as_uuid().as_bytes())?;
        if !self.decisions.insert(*c.decision_id.as_uuid().as_bytes()) {
            return Err(invalid("ambiguous judicial decision identity"));
        }
        Ok(())
    }
    fn operation(&mut self, id: [u8; 16]) -> Result<(), ApplicationError> {
        if !self.operations.insert(id) {
            return Err(invalid("ambiguous operation ownership"));
        }
        Ok(())
    }
    fn member(&mut self, key: MemberKey, value: Member) -> Result<(), ApplicationError> {
        if self.members.insert(key, value).is_some() {
            return Err(invalid("ambiguous measure revision ownership"));
        }
        Ok(())
    }
    pub fn candidate(&self, c: &MeasureAdministrativeCommand) -> Result<(), ApplicationError> {
        if self
            .operations
            .contains(c.operation_id.as_uuid().as_bytes())
        {
            return Err(invalid("administrative operation already has an owner"));
        }
        let revision = c
            .target
            .revision()
            .next()
            .ok_or_else(|| invalid("measure revision overflow"))?;
        if self
            .members
            .contains_key(&(*c.target.id().as_uuid().as_bytes(), revision.get()))
        {
            return Err(invalid("administrative result already has an owner"));
        }
        Ok(())
    }
    pub fn decision_candidate(
        &self,
        c: &MeasureDecisionCommand,
        exclude: Option<usize>,
    ) -> Result<(), ApplicationError> {
        if exclude.is_none()
            && (self
                .operations
                .contains(c.operation_id.as_uuid().as_bytes())
                || self.decisions.contains(c.decision_id.as_uuid().as_bytes()))
        {
            return Err(invalid("decision or operation already owned"));
        }
        let mut new_ids = Vec::new();
        for effect in c.outcome.changes().unwrap_or(&[]) {
            match effect {
                MeasureEffect::Impose(p) => new_ids.push(p.id),
                MeasureEffect::Substitute { successors, .. } => {
                    new_ids.extend(successors.iter().map(|p| p.id))
                }
                _ => {}
            }
        }
        for ((id, revision), m) in &self.members {
            // Later revisions retain their original identity; only a fresh root reserves it.
            if (exclude.is_none() || *revision == 1)
                && Some(self.owner(*m)) != exclude
                && new_ids.iter().any(|n| n.as_uuid().as_bytes() == id)
            {
                return Err(invalid("new measure identity already owned"));
            }
        }
        Ok(())
    }
    pub fn result_available(
        &self,
        r: PrecautionaryMeasureRef,
        exclude: Option<usize>,
    ) -> Result<(), ApplicationError> {
        if let Some(m) = self
            .members
            .get(&(*r.id().as_uuid().as_bytes(), r.revision().get()))
        {
            if Some(self.owner(*m)) != exclude {
                return Err(invalid("result revision already owned"));
            }
        }
        Ok(())
    }
    pub fn selected(&self, r: PrecautionaryMeasureRef) -> Result<Member, ApplicationError> {
        let member = *self
            .members
            .get(&(*r.id().as_uuid().as_bytes(), r.revision().get()))
            .ok_or_else(|| invalid("missing exact record owner"))?;
        let digest = match member {
            Member::Judicial(g, m) => {
                self.evidence.judicial.groups[g].capture.measures[m].capture_digest
            }
            Member::Decision(g, m) => self.evidence.decisions[g].capture.measures[m].capture_digest,
            Member::Administrative(a) => {
                self.evidence.administrative[a].capture.records[0].capture_digest
            }
        };
        if digest != r.digest() {
            return Err(invalid("record digest differs"));
        }
        Ok(member)
    }
    pub fn judicial(&self, r: &MeasureJudicialRef) -> Result<JudicialView<'a>, ApplicationError> {
        let view = match self.selected(r.reference)? {
            Member::Judicial(g, m) => {
                JudicialView::V1(&self.evidence.judicial.groups[g].capture, m)
            }
            Member::Decision(g, m) => JudicialView::V2(&self.evidence.decisions[g].capture, m),
            Member::Administrative(_) => {
                return Err(invalid("judicial reference selects administrative row"))
            }
        };
        if r.owner != view.owner() {
            return Err(invalid("last judicial owner differs"));
        }
        Ok(view)
    }
    pub fn view(&self, member: Member) -> Result<RecordView<'a>, ApplicationError> {
        let (judicial, administrative) = match member {
            Member::Judicial(g, m) => (
                JudicialView::V1(&self.evidence.judicial.groups[g].capture, m),
                None,
            ),
            Member::Decision(g, m) => (
                JudicialView::V2(&self.evidence.decisions[g].capture, m),
                None,
            ),
            Member::Administrative(a) => {
                let a = &self.evidence.administrative[a].capture;
                (self.judicial(&a.records[0].result.last_judicial)?, Some(a))
            }
        };
        Ok(RecordView {
            judicial,
            administrative,
        })
    }
    pub fn owner(&self, m: Member) -> usize {
        match m {
            Member::Judicial(g, _) => g,
            Member::Decision(g, _) => self.evidence.judicial.groups.len() + g,
            Member::Administrative(a) => {
                self.evidence.judicial.groups.len() + self.evidence.decisions.len() + a
            }
        }
    }
    pub fn owners(&self) -> usize {
        self.evidence.judicial.groups.len()
            + self.evidence.decisions.len()
            + self.evidence.administrative.len()
    }
}
