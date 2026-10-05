use super::{record_view::RecordView, wire::invalid, *};
use crate::{precautionary_measures::*, ApplicationError};
use domain::{cases::CaseId, precautionary_hearings::PrecautionaryMeasureRef};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(super) struct HistoryView<'a> {
    pub judicial: &'a MeasureHistoryEvidence,
    pub administrative: &'a [MeasureAdministrativeEvidence],
}
impl<'a> From<&'a MeasureRecordHistoryEvidence> for HistoryView<'a> {
    fn from(e: &'a MeasureRecordHistoryEvidence) -> Self {
        Self {
            judicial: &e.judicial,
            administrative: &e.administrative,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum Member {
    Judicial(usize, usize),
    Administrative(usize),
}
type MemberKey = ([u8; 16], u32);
pub(super) struct RecordIndex<'a> {
    pub evidence: HistoryView<'a>,
    operations: BTreeSet<[u8; 16]>,
    members: BTreeMap<MemberKey, Member>,
}

pub(super) fn bounds(e: HistoryView<'_>, reserve: usize) -> Result<(), ApplicationError> {
    let owners = e
        .judicial
        .groups
        .len()
        .checked_add(e.administrative.len())
        .and_then(|n| n.checked_add(reserve))
        .ok_or_else(|| invalid("owner count overflow"))?;
    if owners > 256 {
        return Err(invalid("combined owner budget exceeded"));
    }
    let mut rows = reserve;
    for g in &e.judicial.groups {
        crate::precautionary_measures::measure_group_shape(&g.capture)?;
        rows = rows
            .checked_add(g.capture.measures.len())
            .ok_or_else(|| invalid("row count overflow"))?;
        if rows > 8192 {
            return Err(invalid("combined row budget exceeded"));
        }
    }
    for a in e.administrative {
        if a.capture.records.len() != 1 {
            return Err(invalid("correction must own exactly one row"));
        }
        rows = rows
            .checked_add(a.capture.records.len())
            .ok_or_else(|| invalid("row count overflow"))?;
        if rows > 8192 {
            return Err(invalid("combined row budget exceeded"));
        }
    }
    Ok(())
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
            members: BTreeMap::new(),
        };
        for (position, g) in evidence.judicial.groups.iter().enumerate() {
            if g.capture.review.case_id != case || g.origin.case_id != case {
                return Err(invalid("judicial case differs"));
            }
            result.operation(*g.capture.review.command.operation_id.as_uuid().as_bytes())?;
            for (row, m) in g.capture.measures.iter().enumerate() {
                result.member(
                    (*m.result.id.as_uuid().as_bytes(), m.result.revision.get()),
                    Member::Judicial(position, row),
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
    pub fn candidate(
        &self,
        command: &MeasureAdministrativeCommand,
    ) -> Result<(), ApplicationError> {
        if self
            .operations
            .contains(command.operation_id.as_uuid().as_bytes())
        {
            return Err(invalid("administrative operation already has an owner"));
        }
        let revision = command
            .target
            .revision()
            .next()
            .ok_or_else(|| invalid("measure revision overflow"))?;
        if self
            .members
            .contains_key(&(*command.target.id().as_uuid().as_bytes(), revision.get()))
        {
            return Err(invalid("administrative result already has an owner"));
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
            Member::Administrative(a) => {
                self.evidence.administrative[a].capture.records[0].capture_digest
            }
        };
        if digest != r.digest() {
            return Err(invalid("record digest differs"));
        }
        Ok(member)
    }
    pub fn judicial(&self, r: &MeasureJudicialRef) -> Result<(usize, usize), ApplicationError> {
        let Member::Judicial(g, m) = self.selected(r.reference)? else {
            return Err(invalid(
                "judicial reference selects an administrative record",
            ));
        };
        let group = &self.evidence.judicial.groups[g].capture;
        if r.owner.operation_id != group.review.command.operation_id
            || r.owner.decision_id != group.review.command.decision_id
            || r.owner.group_digest != group.capture_digest
        {
            return Err(invalid("last judicial owner differs"));
        }
        Ok((g, m))
    }
    pub fn view(&self, member: Member) -> Result<RecordView<'a>, ApplicationError> {
        let (g, m, a) = match member {
            Member::Judicial(g, m) => (g, m, None),
            Member::Administrative(a) => {
                let capture = &self.evidence.administrative[a].capture;
                let (g, m) = self.judicial(&capture.records[0].result.last_judicial)?;
                (g, m, Some(capture))
            }
        };
        Ok(RecordView {
            group: &self.evidence.judicial.groups[g].capture,
            member: m,
            administrative: a,
        })
    }
}
